//! iOS simulator support built on `xcrun simctl` (macOS only).

use std::cmp::Reverse;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;

use crate::model::{Device, DeviceState, Platform};
use crate::process::{format_command, Runner};
use crate::recording::{interrupt, move_file, scratch_dir, Recording};
use crate::tools::Tools;

#[derive(Deserialize)]
struct SimctlList {
    devices: HashMap<String, Vec<SimDevice>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SimDevice {
    udid: String,
    name: String,
    state: String,
    #[serde(default = "default_true")]
    is_available: bool,
}

fn default_true() -> bool {
    true
}

/// `com.apple.CoreSimulator.SimRuntime.iOS-17-2` -> `iOS 17.2`.
pub fn runtime_name(id: &str) -> String {
    let last = id.rsplit('.').next().unwrap_or(id);
    let (os, version) = match last.split_once('-') {
        Some((os, rest)) => (os, rest.replace('-', ".")),
        None => (last, String::new()),
    };
    let os = if os == "xrOS" { "visionOS" } else { os };
    if version.is_empty() { os.to_string() } else { format!("{os} {version}") }
}

fn runtime_sort_key(runtime: &str) -> (u8, Reverse<Vec<u32>>) {
    let (os, version) = runtime.split_once(' ').unwrap_or((runtime, ""));
    let rank = match os {
        "iOS" => 0,
        "visionOS" => 1,
        "tvOS" => 2,
        "watchOS" => 3,
        _ => 4,
    };
    let version = version.split('.').filter_map(|p| p.parse().ok()).collect();
    (rank, Reverse(version))
}

fn map_state(state: &str) -> DeviceState {
    match state {
        "Booted" => DeviceState::Booted,
        "Booting" | "Creating" => DeviceState::Booting,
        "Shutting Down" => DeviceState::ShuttingDown,
        _ => DeviceState::Shutdown,
    }
}

pub fn parse_devices(json: &str) -> Result<Vec<Device>, String> {
    let list: SimctlList =
        serde_json::from_str(json).map_err(|e| format!("Cannot parse simctl output: {e}"))?;
    let mut devices: Vec<Device> = list
        .devices
        .into_iter()
        .flat_map(|(runtime_id, devices)| {
            let runtime = runtime_name(&runtime_id);
            devices.into_iter().filter(|d| d.is_available).map(move |d| Device {
                id: d.udid,
                name: d.name,
                platform: Platform::Ios,
                state: map_state(&d.state),
                runtime: Some(runtime.clone()),
                detail: None,
                serial: None,
                recording: false,
            })
        })
        .collect();
    devices.sort_by(|a, b| {
        let ra = runtime_sort_key(a.runtime.as_deref().unwrap_or_default());
        let rb = runtime_sort_key(b.runtime.as_deref().unwrap_or_default());
        ra.cmp(&rb).then_with(|| a.name.cmp(&b.name))
    });
    Ok(devices)
}

pub async fn list_devices(runner: &Runner, tools: &Tools) -> Result<Vec<Device>, String> {
    let out = runner
        .run_quiet(tools.xcrun()?, &["simctl", "list", "devices", "available", "--json"])
        .await?;
    if !out.success {
        return Err(crate::process::failure_message(Path::new("simctl"), &out));
    }
    parse_devices(&out.stdout_str())
}

pub async fn boot(runner: &Runner, tools: &Tools, udid: &str) -> Result<(), String> {
    let out = runner.run(tools.xcrun()?, &["simctl", "boot", udid]).await?;
    if !out.success && !out.combined().contains("current state: Booted") {
        return Err(crate::process::failure_message(Path::new("simctl"), &out));
    }
    // Bring Simulator.app to the front so the booted device is visible.
    let _ = runner.run(Path::new("/usr/bin/open"), &["-a", "Simulator"]).await;
    Ok(())
}

pub async fn shutdown(runner: &Runner, tools: &Tools, udid: &str) -> Result<(), String> {
    runner.run_checked(tools.xcrun()?, &["simctl", "shutdown", udid]).await.map(|_| ())
}

pub async fn erase(runner: &Runner, tools: &Tools, udid: &str) -> Result<(), String> {
    runner.run_checked(tools.xcrun()?, &["simctl", "erase", udid]).await.map(|_| ())
}

pub async fn open_url(runner: &Runner, tools: &Tools, udid: &str, url: &str) -> Result<(), String> {
    runner.run_checked(tools.xcrun()?, &["simctl", "openurl", udid, url]).await.map(|_| ())
}

pub async fn screenshot(runner: &Runner, tools: &Tools, udid: &str, dest: &Path) -> Result<(), String> {
    let dest = dest.to_str().ok_or("Output path is not valid UTF-8")?;
    runner
        .run_checked(tools.xcrun()?, &["simctl", "io", udid, "screenshot", "--type=png", dest])
        .await
        .map(|_| ())
}

pub async fn install(runner: &Runner, tools: &Tools, udid: &str, app: &str) -> Result<(), String> {
    runner.run_long(tools.xcrun()?, &["simctl", "install", udid, app]).await.and_then(|out| {
        if out.success {
            Ok(())
        } else {
            Err(crate::process::failure_message(Path::new("simctl"), &out))
        }
    })
}

pub async fn start_recording(runner: &Runner, tools: &Tools, udid: &str) -> Result<Recording, String> {
    // Spawn simctl directly (not via xcrun) so SIGINT reaches the recorder.
    let found = runner.run_quiet(tools.xcrun()?, &["--find", "simctl"]).await?;
    let simctl = PathBuf::from(found.stdout_str().trim());
    if !simctl.is_file() {
        return Err("Could not locate simctl".into());
    }

    let temp = scratch_dir().join(format!("recording-{udid}.mp4"));
    let temp_str = temp.to_str().ok_or("Temp path is not valid UTF-8")?.to_string();
    let args = ["io", udid, "recordVideo", "--codec=h264", "--force", temp_str.as_str()];
    let mut cmd = runner.command(&simctl, &args);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    let display = format_command(&simctl, &args);
    let mut child = cmd.spawn().map_err(|e| format!("Failed to start recording: {e}"))?;

    if tokio::time::timeout(Duration::from_millis(1500), child.wait()).await.is_ok() {
        let details = child
            .wait_with_output()
            .await
            .map(|o| String::from_utf8_lossy(&o.stderr).into_owned())
            .unwrap_or_default();
        runner.log(&display, false, &details);
        return Err(format!("Recording failed to start. {}", details.trim()));
    }
    runner.log(&display, true, "Recording…");
    Ok(Recording { child, temp_path: Some(temp), remote_path: None, serial: None })
}

pub async fn stop_recording(runner: &Runner, mut rec: Recording, dest: &Path) -> Result<(), String> {
    let temp = rec.temp_path.clone().ok_or("Missing recording path")?;
    interrupt(&rec.child);
    if tokio::time::timeout(Duration::from_secs(15), rec.child.wait()).await.is_err() {
        let _ = rec.child.kill().await;
        return Err("The recorder did not stop in time; the video may be incomplete.".into());
    }
    move_file(&temp, dest).map_err(|e| format!("Cannot save recording: {e}"))?;
    runner.log("simctl io recordVideo", true, &format!("Saved {}", dest.display()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_runtime_names() {
        assert_eq!(runtime_name("com.apple.CoreSimulator.SimRuntime.iOS-17-2"), "iOS 17.2");
        assert_eq!(runtime_name("com.apple.CoreSimulator.SimRuntime.watchOS-10-0"), "watchOS 10.0");
        assert_eq!(runtime_name("com.apple.CoreSimulator.SimRuntime.xrOS-1-0"), "visionOS 1.0");
    }

    #[test]
    fn parses_and_sorts_simctl_json() {
        let json = r#"{"devices": {
            "com.apple.CoreSimulator.SimRuntime.iOS-16-4": [
                {"udid": "A", "name": "iPhone 14", "state": "Shutdown", "isAvailable": true}
            ],
            "com.apple.CoreSimulator.SimRuntime.watchOS-10-2": [
                {"udid": "W", "name": "Apple Watch", "state": "Shutdown", "isAvailable": true}
            ],
            "com.apple.CoreSimulator.SimRuntime.iOS-17-2": [
                {"udid": "C", "name": "iPhone 15 Pro", "state": "Booted", "isAvailable": true},
                {"udid": "B", "name": "iPad Air", "state": "Shutdown", "isAvailable": true},
                {"udid": "X", "name": "Broken", "state": "Shutdown", "isAvailable": false}
            ]
        }}"#;
        let devices = parse_devices(json).unwrap();
        let ids: Vec<&str> = devices.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids, vec!["B", "C", "A", "W"]);
        assert_eq!(devices[1].state, DeviceState::Booted);
        assert_eq!(devices[0].runtime.as_deref(), Some("iOS 17.2"));
    }
}
