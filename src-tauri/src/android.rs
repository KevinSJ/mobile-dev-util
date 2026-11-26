//! Android emulator support built on `emulator` and `adb`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;

use crate::model::{Device, DeviceState, Platform};
use crate::process::{format_command, Runner};
use crate::recording::{scratch_dir, Recording};
use crate::tools::{home_dir, Tools};

// ---------------------------------------------------------------------------
// Parsing helpers (pure, unit tested)
// ---------------------------------------------------------------------------

/// AVD names may only contain `[A-Za-z0-9._-]`.
pub fn is_valid_avd_name(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Parses `emulator -list-avds`, ignoring the INFO/WARNING lines newer
/// emulator builds print to stdout.
pub fn parse_avd_list(out: &str) -> Vec<String> {
    out.lines()
        .map(str::trim)
        .filter(|l| is_valid_avd_name(l))
        .map(String::from)
        .collect()
}

/// Parses `adb devices`, returning `(serial, state)` for emulators only.
pub fn parse_adb_devices(out: &str) -> Vec<(String, String)> {
    out.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let serial = parts.next()?;
            let state = parts.next()?;
            serial
                .starts_with("emulator-")
                .then(|| (serial.to_string(), state.to_string()))
        })
        .collect()
}

/// Parses the output of `adb emu avd name` (`<name>\r\nOK`).
pub fn parse_emu_avd_name(out: &str) -> Option<String> {
    out.lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && *l != "OK" && is_valid_avd_name(l))
        .map(String::from)
}

pub fn parse_ini(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

#[derive(Debug, Default, PartialEq)]
pub struct AvdInfo {
    pub display_name: Option<String>,
    pub runtime: Option<String>,
    pub detail: Option<String>,
}

fn api_from_sysdir(sysdir: &str) -> Option<String> {
    sysdir.split(['/', '\\']).find_map(|seg| seg.strip_prefix("android-")).map(|v| {
        if v.starts_with(|c: char| c.is_ascii_digit()) {
            format!("API {v}")
        } else {
            format!("Android {v}")
        }
    })
}

pub fn summarize_config(config: &HashMap<String, String>) -> AvdInfo {
    let get = |k: &str| config.get(k).map(|s| s.trim()).filter(|s| !s.is_empty());
    let detail: Vec<&str> = [get("tag.display"), get("abi.type")].into_iter().flatten().collect();
    AvdInfo {
        display_name: get("avd.ini.displayname").map(String::from),
        runtime: get("image.sysdir.1").and_then(api_from_sysdir),
        detail: (!detail.is_empty()).then(|| detail.join(" · ")),
    }
}

/// Quotes a value for the device-side shell that `adb shell` hands its
/// arguments to.
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

// ---------------------------------------------------------------------------
// AVD metadata on disk
// ---------------------------------------------------------------------------

pub fn avd_home() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("ANDROID_AVD_HOME").filter(|p| !p.is_empty()) {
        return Some(p.into());
    }
    for var in ["ANDROID_USER_HOME", "ANDROID_EMULATOR_HOME"] {
        if let Some(p) = std::env::var_os(var).filter(|p| !p.is_empty()) {
            return Some(PathBuf::from(p).join("avd"));
        }
    }
    home_dir().map(|h| h.join(".android").join("avd"))
}

fn read_avd_info(home: &Path, name: &str) -> AvdInfo {
    let ini = fs::read_to_string(home.join(format!("{name}.ini")))
        .map(|t| parse_ini(&t))
        .unwrap_or_default();
    let avd_dir = ini
        .get("path")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| home.join(format!("{name}.avd")));
    let config = fs::read_to_string(avd_dir.join("config.ini"))
        .map(|t| parse_ini(&t))
        .unwrap_or_default();
    summarize_config(&config)
}

fn scan_avd_home(home: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(home)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let file = e.file_name().to_string_lossy().into_owned();
            file.strip_suffix(".ini").filter(|n| is_valid_avd_name(n)).map(String::from)
        })
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names
}

// ---------------------------------------------------------------------------
// Device discovery
// ---------------------------------------------------------------------------

struct Running {
    serial: String,
    avd: Option<String>,
    state: DeviceState,
}

async fn avd_name_for(runner: &Runner, adb: &Path, serial: &str) -> Option<String> {
    if let Ok(out) = runner.run_quiet(adb, &["-s", serial, "emu", "avd", "name"]).await {
        if let Some(name) = parse_emu_avd_name(&out.stdout_str()) {
            return Some(name);
        }
    }
    for prop in ["ro.boot.qemu.avd_name", "ro.kernel.qemu.avd_name"] {
        if let Ok(out) = runner.run_quiet(adb, &["-s", serial, "shell", "getprop", prop]).await {
            let name = out.stdout_str().trim().to_string();
            if is_valid_avd_name(&name) {
                return Some(name);
            }
        }
    }
    None
}

async fn running_emulators(
    runner: &Runner,
    adb: &Path,
    names: &Mutex<HashMap<String, String>>,
) -> Vec<Running> {
    let Ok(out) = runner.run_quiet(adb, &["devices"]).await else {
        return Vec::new();
    };
    let devices = parse_adb_devices(&out.stdout_str());

    // Serial -> AVD name is stable while an emulator runs, so cache it.
    names
        .lock()
        .unwrap()
        .retain(|serial, _| devices.iter().any(|(s, _)| s == serial));

    let mut running = Vec::new();
    for (serial, adb_state) in devices {
        let cached = names.lock().unwrap().get(&serial).cloned();
        let avd = match cached {
            Some(name) => Some(name),
            None => {
                let name = avd_name_for(runner, adb, &serial).await;
                if let Some(name) = &name {
                    names.lock().unwrap().insert(serial.clone(), name.clone());
                }
                name
            }
        };
        let state = if adb_state == "device" {
            let booted = runner
                .run_quiet(adb, &["-s", &serial, "shell", "getprop", "sys.boot_completed"])
                .await
                .map(|o| o.stdout_str().trim() == "1")
                .unwrap_or(false);
            if booted { DeviceState::Booted } else { DeviceState::Booting }
        } else {
            DeviceState::Booting
        };
        running.push(Running { serial, avd, state });
    }
    running
}

pub async fn list_devices(
    runner: &Runner,
    tools: &Tools,
    names_cache: &Mutex<HashMap<String, String>>,
) -> Result<Vec<Device>, String> {
    if tools.emulator.is_none() && tools.adb.is_none() {
        return Err(
            "Android SDK not found. Install Android Studio or set the SDK path in Settings.".into(),
        );
    }
    let home = avd_home();

    let mut names = match &tools.emulator {
        Some(emulator) => match runner.run_quiet(emulator, &["-list-avds"]).await {
            Ok(out) => parse_avd_list(&out.stdout_str()),
            Err(_) => Vec::new(),
        },
        None => Vec::new(),
    };
    if names.is_empty() {
        if let Some(home) = &home {
            names = scan_avd_home(home);
        }
    }

    let running = match &tools.adb {
        Some(adb) => running_emulators(runner, adb, names_cache).await,
        None => Vec::new(),
    };

    // Emulators running an AVD we couldn't enumerate still deserve a row.
    for r in &running {
        let id = r.avd.clone().unwrap_or_else(|| r.serial.clone());
        if !names.contains(&id) {
            names.push(id);
        }
    }

    Ok(names
        .into_iter()
        .map(|id| {
            let info = home.as_deref().map(|h| read_avd_info(h, &id)).unwrap_or_default();
            let live = running
                .iter()
                .find(|r| r.avd.as_deref() == Some(id.as_str()) || r.serial == id);
            Device {
                name: info.display_name.unwrap_or_else(|| id.replace('_', " ")),
                platform: Platform::Android,
                state: live.map(|r| r.state).unwrap_or(DeviceState::Shutdown),
                runtime: info.runtime,
                detail: info.detail,
                serial: live.map(|r| r.serial.clone()),
                recording: false,
                id,
            }
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

fn tail(path: &Path, lines: usize) -> String {
    let text = fs::read_to_string(path).unwrap_or_default();
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

/// Launches the emulator as a detached process that outlives this app.
pub async fn boot(runner: &Runner, tools: &Tools, avd: &str, extra: &[&str]) -> Result<(), String> {
    if !is_valid_avd_name(avd) {
        return Err(format!("Invalid AVD name: {avd}"));
    }
    let emulator = tools.emulator()?;
    let mut args = vec!["-avd", avd];
    args.extend_from_slice(extra);

    let log_path = scratch_dir().join(format!("emulator-{avd}.log"));
    let log = fs::File::create(&log_path).map_err(|e| format!("Cannot create log file: {e}"))?;
    let log_err = log.try_clone().map_err(|e| e.to_string())?;

    let mut cmd = runner.command(emulator, &args);
    cmd.stdout(Stdio::from(log)).stderr(Stdio::from(log_err));
    // Own process group so Ctrl+C in `tauri dev` / quitting the app doesn't kill it.
    #[cfg(unix)]
    cmd.process_group(0);
    #[cfg(windows)]
    cmd.creation_flags(crate::process::CREATE_NO_WINDOW | 0x0000_0200 /* CREATE_NEW_PROCESS_GROUP */);

    let display = format_command(emulator, &args);
    let mut child = cmd.spawn().map_err(|e| format!("Failed to start emulator: {e}"))?;

    // A misconfigured AVD typically dies within a second or two.
    match tokio::time::timeout(Duration::from_secs(3), child.wait()).await {
        Ok(status) => {
            let details = tail(&log_path, 15);
            let status = status.map(|s| s.to_string()).unwrap_or_else(|e| e.to_string());
            runner.log(&display, false, &details);
            Err(format!("The emulator exited immediately ({status}).\n{details}"))
        }
        Err(_) => {
            runner.log(&display, true, &format!("Started (log: {})", log_path.display()));
            tauri::async_runtime::spawn(async move {
                let _ = child.wait().await;
            });
            Ok(())
        }
    }
}

pub async fn shutdown(runner: &Runner, tools: &Tools, serial: &str) -> Result<(), String> {
    runner.run_checked(tools.adb()?, &["-s", serial, "emu", "kill"]).await.map(|_| ())
}

pub async fn open_url(runner: &Runner, tools: &Tools, serial: &str, url: &str) -> Result<(), String> {
    let quoted = shell_quote(url);
    let out = runner
        .run_checked(
            tools.adb()?,
            &["-s", serial, "shell", "am", "start", "-a", "android.intent.action.VIEW", "-d", &quoted],
        )
        .await?;
    // `am start` exits 0 even when nothing can handle the intent.
    let text = out.combined();
    if text.contains("Error:") || text.contains("Exception") {
        return Err(text);
    }
    Ok(())
}

pub async fn screenshot(runner: &Runner, tools: &Tools, serial: &str, dest: &Path) -> Result<(), String> {
    let out = runner
        .run_checked(tools.adb()?, &["-s", serial, "exec-out", "screencap", "-p"])
        .await?;
    if !out.stdout.starts_with(b"\x89PNG") {
        return Err(format!("Unexpected screencap output: {}", out.combined()));
    }
    fs::write(dest, &out.stdout).map_err(|e| format!("Cannot write {}: {e}", dest.display()))
}

pub async fn install(runner: &Runner, tools: &Tools, serial: &str, apk: &str) -> Result<(), String> {
    let out = runner.run_long(tools.adb()?, &["-s", serial, "install", "-r", "-t", apk]).await?;
    let text = out.combined();
    if out.success && text.contains("Success") {
        Ok(())
    } else {
        Err(if text.is_empty() { "adb install failed".into() } else { text })
    }
}

pub async fn start_recording(runner: &Runner, tools: &Tools, serial: &str) -> Result<Recording, String> {
    let adb = tools.adb()?;
    let remote = format!(
        "/sdcard/mdu_record_{}.mp4",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    );
    let args = ["-s", serial, "shell", "screenrecord", "--bit-rate", "8000000", remote.as_str()];
    let mut cmd = runner.command(adb, &args);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    let display = format_command(adb, &args);
    let mut child = cmd.spawn().map_err(|e| format!("Failed to start screenrecord: {e}"))?;

    if let Ok(status) = tokio::time::timeout(Duration::from_millis(1200), child.wait()).await {
        let out = child.wait_with_output().await.ok();
        let details = out
            .map(|o| {
                format!(
                    "{}{}",
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                )
            })
            .unwrap_or_default();
        runner.log(&display, false, &details);
        let status = status.map(|s| s.to_string()).unwrap_or_default();
        return Err(format!("screenrecord exited immediately ({status}). {}", details.trim()));
    }
    runner.log(&display, true, "Recording… (Android limits recordings to 3 minutes)");
    Ok(Recording {
        child,
        temp_path: None,
        remote_path: Some(remote),
        serial: Some(serial.to_string()),
    })
}

pub async fn stop_recording(
    runner: &Runner,
    tools: &Tools,
    mut rec: Recording,
    dest: &Path,
) -> Result<(), String> {
    let adb = tools.adb()?;
    let serial = rec.serial.clone().ok_or("Missing device serial")?;
    let remote = rec.remote_path.clone().ok_or("Missing remote path")?;

    // Interrupt screenrecord on the device so it writes the MP4 trailer.
    let _ = runner.run(adb, &["-s", &serial, "shell", "pkill", "-INT", "screenrecord"]).await;
    if tokio::time::timeout(Duration::from_secs(10), rec.child.wait()).await.is_err() {
        let _ = rec.child.kill().await;
    }
    tokio::time::sleep(Duration::from_millis(800)).await;

    let dest_str = dest.to_str().ok_or("Output path is not valid UTF-8")?;
    let pulled = runner.run_long(adb, &["-s", &serial, "pull", &remote, dest_str]).await;
    let _ = runner.run_quiet(adb, &["-s", &serial, "shell", "rm", "-f", &remote]).await;
    let out = pulled?;
    if out.success && dest.is_file() {
        Ok(())
    } else {
        Err(crate::process::failure_message(adb, &out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_avd_list_and_skips_noise() {
        let out = "INFO    | Storing crashdata in: /tmp/foo\nPixel_7_API_34\nTablet.API-33\n\n";
        assert_eq!(parse_avd_list(out), vec!["Pixel_7_API_34", "Tablet.API-33"]);
    }

    #[test]
    fn parses_adb_devices() {
        let out = "List of devices attached\nemulator-5554\tdevice\nemulator-5556\toffline\nR58M123\tdevice\n\n";
        assert_eq!(
            parse_adb_devices(out),
            vec![
                ("emulator-5554".to_string(), "device".to_string()),
                ("emulator-5556".to_string(), "offline".to_string())
            ]
        );
    }

    #[test]
    fn parses_emu_avd_name() {
        assert_eq!(parse_emu_avd_name("Pixel_7_API_34\r\nOK\r\n").as_deref(), Some("Pixel_7_API_34"));
        assert_eq!(parse_emu_avd_name("OK\r\n"), None);
    }

    #[test]
    fn summarizes_config_ini() {
        let config = parse_ini(
            "avd.ini.displayname=Pixel 7 API 34\nimage.sysdir.1=system-images/android-34/google_apis_playstore/arm64-v8a/\ntag.display=Google Play\nabi.type=arm64-v8a\n",
        );
        assert_eq!(
            summarize_config(&config),
            AvdInfo {
                display_name: Some("Pixel 7 API 34".into()),
                runtime: Some("API 34".into()),
                detail: Some("Google Play · arm64-v8a".into()),
            }
        );
        assert_eq!(api_from_sysdir("system-images/android-VanillaIceCream/x86_64/").as_deref(), Some("Android VanillaIceCream"));
    }

    #[test]
    fn quotes_for_device_shell() {
        assert_eq!(shell_quote("myapp://a?b=1&c=2"), "'myapp://a?b=1&c=2'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }
}
