// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod android;
mod ios;
mod model;
mod process;
mod recording;
mod tools;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use model::{file_stem, Device, DeviceTarget, Platform};
use process::Runner;
use recording::Recording;
use tools::Tools;

#[derive(Default)]
struct AppState {
    sdk_override: Mutex<Option<PathBuf>>,
    /// adb serial -> AVD name for running emulators.
    avd_names: Mutex<HashMap<String, String>>,
    /// Active recordings keyed by `DeviceTarget::key`.
    recordings: Mutex<HashMap<String, Recording>>,
}

impl AppState {
    fn tools(&self) -> Tools {
        tools::resolve(self.sdk_override.lock().unwrap().as_deref())
    }
}

fn context(app: &AppHandle, state: &AppState) -> (Tools, Runner) {
    let tools = state.tools();
    let runner = Runner::new(app, tools.envs());
    (tools, runner)
}

fn path_string(p: &Option<PathBuf>) -> Option<String> {
    p.as_ref().map(|p| p.display().to_string())
}

fn default_output_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().desktop_dir().or_else(|_| app.path().home_dir()).ok()
}

/// Resolves (and creates) the folder screenshots/recordings are saved to.
fn output_file(app: &AppHandle, dir: Option<String>, name: &str, ext: &str) -> Result<PathBuf, String> {
    let dir = match dir.filter(|d| !d.trim().is_empty()) {
        Some(d) => PathBuf::from(d),
        None => default_output_dir(app).ok_or("Cannot determine an output folder")?,
    };
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create {}: {e}", dir.display()))?;
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    Ok(dir.join(format!("{}_{stamp}.{ext}", file_stem(name))))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Environment {
    os: &'static str,
    android_sdk: Option<String>,
    adb: Option<String>,
    emulator: Option<String>,
    xcrun: Option<String>,
    sdk_override: Option<String>,
    default_output_dir: Option<String>,
    home_dir: Option<String>,
}

#[tauri::command]
fn get_environment(app: AppHandle, state: State<'_, AppState>) -> Environment {
    let tools = state.tools();
    Environment {
        os: std::env::consts::OS,
        android_sdk: path_string(&tools.android_sdk),
        adb: path_string(&tools.adb),
        emulator: path_string(&tools.emulator),
        xcrun: path_string(&tools.xcrun),
        sdk_override: path_string(&state.sdk_override.lock().unwrap()),
        default_output_dir: path_string(&default_output_dir(&app)),
        home_dir: path_string(&app.path().home_dir().ok()),
    }
}

#[tauri::command]
fn set_android_sdk(app: AppHandle, state: State<'_, AppState>, path: Option<String>) -> Environment {
    *state.sdk_override.lock().unwrap() = path.filter(|p| !p.trim().is_empty()).map(PathBuf::from);
    state.avd_names.lock().unwrap().clear();
    get_environment(app, state)
}

#[tauri::command]
async fn list_devices(
    app: AppHandle,
    state: State<'_, AppState>,
    platform: Platform,
) -> Result<Vec<Device>, String> {
    let (tools, runner) = context(&app, &state);
    let mut devices = match platform {
        Platform::Android => android::list_devices(&runner, &tools, &state.avd_names).await?,
        Platform::Ios => ios::list_devices(&runner, &tools).await?,
    };
    let recordings = state.recordings.lock().unwrap();
    for d in &mut devices {
        d.recording = recordings.contains_key(&format!("{}:{}", platform.key(), d.id));
    }
    Ok(devices)
}

#[tauri::command]
async fn boot_device(
    app: AppHandle,
    state: State<'_, AppState>,
    target: DeviceTarget,
    cold_boot: bool,
) -> Result<(), String> {
    let (tools, runner) = context(&app, &state);
    match target.platform {
        Platform::Android => {
            let extra: &[&str] = if cold_boot { &["-no-snapshot-load"] } else { &[] };
            android::boot(&runner, &tools, &target.id, extra).await
        }
        Platform::Ios => ios::boot(&runner, &tools, &target.id).await,
    }
}

#[tauri::command]
async fn shutdown_device(app: AppHandle, state: State<'_, AppState>, target: DeviceTarget) -> Result<(), String> {
    let (tools, runner) = context(&app, &state);
    match target.platform {
        Platform::Android => android::shutdown(&runner, &tools, target.serial()?).await,
        Platform::Ios => ios::shutdown(&runner, &tools, &target.id).await,
    }
}

/// Android: boots with `-wipe-data`. iOS: `simctl erase` (device must be shut down).
#[tauri::command]
async fn wipe_device(app: AppHandle, state: State<'_, AppState>, target: DeviceTarget) -> Result<(), String> {
    let (tools, runner) = context(&app, &state);
    match target.platform {
        Platform::Android => {
            if target.serial.is_some() {
                return Err("Shut the emulator down before wiping its data.".into());
            }
            android::boot(&runner, &tools, &target.id, &["-wipe-data"]).await
        }
        Platform::Ios => ios::erase(&runner, &tools, &target.id).await,
    }
}

#[tauri::command]
async fn open_url(
    app: AppHandle,
    state: State<'_, AppState>,
    target: DeviceTarget,
    url: String,
) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("Enter a URL or deep link first.".into());
    }
    let (tools, runner) = context(&app, &state);
    match target.platform {
        Platform::Android => android::open_url(&runner, &tools, target.serial()?, url).await,
        Platform::Ios => ios::open_url(&runner, &tools, &target.id, url).await,
    }
}

#[tauri::command]
async fn take_screenshot(
    app: AppHandle,
    state: State<'_, AppState>,
    target: DeviceTarget,
    output_dir: Option<String>,
) -> Result<String, String> {
    let (tools, runner) = context(&app, &state);
    let dest = output_file(&app, output_dir, &target.name, "png")?;
    match target.platform {
        Platform::Android => android::screenshot(&runner, &tools, target.serial()?, &dest).await?,
        Platform::Ios => ios::screenshot(&runner, &tools, &target.id, &dest).await?,
    }
    Ok(dest.display().to_string())
}

#[tauri::command]
async fn install_app(
    app: AppHandle,
    state: State<'_, AppState>,
    target: DeviceTarget,
    path: String,
) -> Result<(), String> {
    let (tools, runner) = context(&app, &state);
    match target.platform {
        Platform::Android => android::install(&runner, &tools, target.serial()?, &path).await,
        Platform::Ios => ios::install(&runner, &tools, &target.id, &path).await,
    }
}

#[tauri::command]
async fn start_recording(app: AppHandle, state: State<'_, AppState>, target: DeviceTarget) -> Result<(), String> {
    let key = target.key();
    if state.recordings.lock().unwrap().contains_key(&key) {
        return Err("Already recording this device.".into());
    }
    let (tools, runner) = context(&app, &state);
    let rec = match target.platform {
        Platform::Android => android::start_recording(&runner, &tools, target.serial()?).await?,
        Platform::Ios => ios::start_recording(&runner, &tools, &target.id).await?,
    };
    state.recordings.lock().unwrap().insert(key, rec);
    Ok(())
}

#[tauri::command]
async fn stop_recording(
    app: AppHandle,
    state: State<'_, AppState>,
    target: DeviceTarget,
    output_dir: Option<String>,
) -> Result<String, String> {
    let rec = state
        .recordings
        .lock()
        .unwrap()
        .remove(&target.key())
        .ok_or("This device is not being recorded.")?;
    let (tools, runner) = context(&app, &state);
    let dest = output_file(&app, output_dir, &target.name, "mp4")?;
    match target.platform {
        Platform::Android => android::stop_recording(&runner, &tools, rec, &dest).await?,
        Platform::Ios => ios::stop_recording(&runner, rec, &dest).await?,
    }
    Ok(dest.display().to_string())
}

#[tauri::command]
fn reveal_path(app: AppHandle, path: String) -> Result<(), String> {
    app.opener().reveal_item_in_dir(path).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_environment,
            set_android_sdk,
            list_devices,
            boot_device,
            shutdown_device,
            wipe_device,
            open_url,
            take_screenshot,
            install_app,
            start_recording,
            stop_recording,
            reveal_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
