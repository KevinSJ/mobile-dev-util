use std::path::PathBuf;

use tokio::process::Child;

/// An in-progress screen recording.
pub struct Recording {
    pub child: Child,
    /// iOS: file being written on the host, moved to the output folder on stop.
    pub temp_path: Option<PathBuf>,
    /// Android: file being written on the device, pulled on stop.
    pub remote_path: Option<String>,
    pub serial: Option<String>,
}

/// Sends SIGINT so the recorder can finalise the video container.
#[cfg(unix)]
pub fn interrupt(child: &Child) -> bool {
    match child.id() {
        // SAFETY: plain syscall on a pid we own.
        Some(pid) => unsafe { libc::kill(pid as libc::pid_t, libc::SIGINT) == 0 },
        None => false,
    }
}

#[cfg(not(unix))]
pub fn interrupt(_child: &Child) -> bool {
    false
}

/// Moves a file, falling back to copy + delete across file systems.
pub fn move_file(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to)?;
    let _ = std::fs::remove_file(from);
    Ok(())
}

/// Directory for scratch files (emulator logs, in-progress recordings).
pub fn scratch_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("mobile-dev-util");
    let _ = std::fs::create_dir_all(&dir);
    dir
}
