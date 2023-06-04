// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_available_emulators_ios(command: String) -> String {
    let output = Command::new("sh").arg("-c").arg(command).output().unwrap();
    return format!("{:?}", String::from_utf8_lossy(&output.stdout));
}


#[tauri::command]
fn get_available_emulators_android(command: String) -> String {
    let output = Command::new("sh").arg("-c").arg(command).output().unwrap();
    return format!("{:?}", String::from_utf8_lossy(&output.stdout));
}

#[tauri::command]
fn run_shell_command_with_result(command: String) -> String {
    let output = Command::new("sh").arg("-c").arg(command).output().unwrap();
    return format!("{:?}", String::from_utf8_lossy(&output.stdout));
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            greet,
            get_available_emulators_android,
            run_shell_command_with_result
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
