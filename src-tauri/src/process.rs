//! Thin wrapper around `tokio::process` that adds timeouts, consistent error
//! messages and an activity feed that the UI can display.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::process::Command;

/// Event name used to stream executed commands to the frontend.
pub const ACTIVITY_EVENT: &str = "activity";

const MAX_LOGGED_OUTPUT: usize = 4_000;

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntry {
    pub timestamp: String,
    pub command: String,
    pub ok: bool,
    pub output: String,
}

pub struct Output {
    pub stdout: Vec<u8>,
    pub stderr: String,
    pub success: bool,
}

impl Output {
    pub fn stdout_str(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    /// stdout + stderr, trimmed — handy for surfacing tool output in errors.
    pub fn combined(&self) -> String {
        let mut text = self.stdout_str().trim().to_string();
        let err = self.stderr.trim();
        if !err.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(err);
        }
        text
    }
}

/// Executes external tools on behalf of a command and reports them to the UI.
pub struct Runner {
    app: AppHandle,
    envs: Vec<(&'static str, PathBuf)>,
}

impl Runner {
    pub fn new(app: &AppHandle, envs: Vec<(&'static str, PathBuf)>) -> Self {
        Self { app: app.clone(), envs }
    }

    /// Builds a command with the resolved SDK environment applied. GUI apps
    /// (especially on macOS) don't inherit the user's shell environment, so the
    /// Android tools would otherwise fail to locate system images.
    pub fn command(&self, program: &Path, args: &[&str]) -> Command {
        let mut cmd = Command::new(program);
        cmd.args(args).stdin(Stdio::null());
        for (key, value) in &self.envs {
            cmd.env(key, value);
        }
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }

    /// Runs a command, logging it to the activity feed (60s timeout).
    pub async fn run(&self, program: &Path, args: &[&str]) -> Result<Output, String> {
        self.exec(program, args, false, Duration::from_secs(60)).await
    }

    /// Runs a command without logging it (used for background polling).
    pub async fn run_quiet(&self, program: &Path, args: &[&str]) -> Result<Output, String> {
        self.exec(program, args, true, Duration::from_secs(15)).await
    }

    /// Runs a potentially slow command such as an app install (10 min timeout).
    pub async fn run_long(&self, program: &Path, args: &[&str]) -> Result<Output, String> {
        self.exec(program, args, false, Duration::from_secs(600)).await
    }

    /// Runs a command and turns a non-zero exit status into an error.
    pub async fn run_checked(&self, program: &Path, args: &[&str]) -> Result<Output, String> {
        let output = self.run(program, args).await?;
        if output.success {
            Ok(output)
        } else {
            Err(failure_message(program, &output))
        }
    }

    async fn exec(
        &self,
        program: &Path,
        args: &[&str],
        quiet: bool,
        timeout: Duration,
    ) -> Result<Output, String> {
        let display = format_command(program, args);
        let mut cmd = self.command(program, args);
        cmd.kill_on_drop(true);

        let output = match tokio::time::timeout(timeout, cmd.output()).await {
            Err(_) => {
                let msg = format!("Timed out after {}s", timeout.as_secs());
                if !quiet {
                    self.log(&display, false, &msg);
                }
                return Err(format!("{display}: {msg}"));
            }
            Ok(Err(err)) => {
                let msg = format!("Failed to start: {err}");
                if !quiet {
                    self.log(&display, false, &msg);
                }
                return Err(format!("{display}: {msg}"));
            }
            Ok(Ok(output)) => Output {
                stdout: output.stdout,
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                success: output.status.success(),
            },
        };

        if !quiet {
            // Binary output (e.g. screenshots) is not useful in the log.
            let mut text = match std::str::from_utf8(&output.stdout) {
                Ok(s) => s.trim().to_string(),
                Err(_) => format!("<{} bytes of binary output>", output.stdout.len()),
            };
            let err = output.stderr.trim();
            if !err.is_empty() {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(err);
            }
            self.log(&display, output.success, &text);
        }
        Ok(output)
    }

    /// Records an entry in the activity feed.
    pub fn log(&self, command: &str, ok: bool, output: &str) {
        let mut output = output.to_string();
        if output.len() > MAX_LOGGED_OUTPUT {
            let mut cut = MAX_LOGGED_OUTPUT;
            while !output.is_char_boundary(cut) {
                cut -= 1;
            }
            output.truncate(cut);
            output.push_str("\n…");
        }
        let entry = ActivityEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            command: command.to_string(),
            ok,
            output,
        };
        let _ = self.app.emit(ACTIVITY_EVENT, entry);
    }
}

pub fn failure_message(program: &Path, output: &Output) -> String {
    let name = program_name(program);
    let details = output.combined();
    if details.is_empty() {
        format!("{name} failed")
    } else {
        format!("{name} failed: {details}")
    }
}

fn program_name(program: &Path) -> String {
    program
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| program.display().to_string())
}

/// Renders a command line for display, quoting arguments where needed.
pub fn format_command(program: &Path, args: &[&str]) -> String {
    let mut parts = vec![program_name(program)];
    for arg in args {
        if arg.is_empty() || arg.chars().any(|c| c.is_whitespace() || c == '"') {
            parts.push(format!("\"{}\"", arg.replace('"', "\\\"")));
        } else {
            parts.push((*arg).to_string());
        }
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_commands_with_quoting() {
        let cmd = format_command(
            Path::new("/opt/sdk/platform-tools/adb"),
            &["-s", "emulator-5554", "install", "/tmp/My App.apk"],
        );
        assert_eq!(cmd, "adb -s emulator-5554 install \"/tmp/My App.apk\"");
    }
}
