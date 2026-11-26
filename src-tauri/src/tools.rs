//! Locates the Android SDK tools and Xcode's `xcrun` without relying on the
//! user's shell `PATH` (which GUI apps on macOS don't inherit).

use std::env;
use std::path::{Path, PathBuf};

const EXE_SUFFIX: &str = if cfg!(windows) { ".exe" } else { "" };

#[derive(Clone, Debug, Default)]
pub struct Tools {
    pub android_sdk: Option<PathBuf>,
    pub adb: Option<PathBuf>,
    pub emulator: Option<PathBuf>,
    pub xcrun: Option<PathBuf>,
}

impl Tools {
    /// Environment variables to pass to spawned tools.
    pub fn envs(&self) -> Vec<(&'static str, PathBuf)> {
        match &self.android_sdk {
            Some(sdk) => vec![("ANDROID_HOME", sdk.clone()), ("ANDROID_SDK_ROOT", sdk.clone())],
            None => Vec::new(),
        }
    }

    pub fn adb(&self) -> Result<&Path, String> {
        self.adb.as_deref().ok_or_else(|| {
            "adb was not found. Install Android SDK Platform-Tools or set the SDK path in Settings."
                .to_string()
        })
    }

    pub fn emulator(&self) -> Result<&Path, String> {
        self.emulator.as_deref().ok_or_else(|| {
            "The Android emulator was not found. Install it via Android Studio's SDK Manager or set the SDK path in Settings."
                .to_string()
        })
    }

    pub fn xcrun(&self) -> Result<&Path, String> {
        self.xcrun.as_deref().ok_or_else(|| {
            if cfg!(target_os = "macos") {
                "xcrun was not found. Install Xcode and its command line tools.".to_string()
            } else {
                "iOS simulators are only available on macOS with Xcode installed.".to_string()
            }
        })
    }
}

pub fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
}

fn env_path(var: &str) -> Option<PathBuf> {
    env::var_os(var).filter(|p| !p.is_empty()).map(PathBuf::from)
}

fn sdk_candidates(sdk_override: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = sdk_override {
        candidates.push(path.to_path_buf());
    }
    candidates.extend(["ANDROID_HOME", "ANDROID_SDK_ROOT"].iter().filter_map(|v| env_path(v)));
    if let Some(home) = home_dir() {
        if cfg!(target_os = "macos") {
            candidates.push(home.join("Library").join("Android").join("sdk"));
        } else if cfg!(not(windows)) {
            candidates.push(home.join("Android").join("Sdk"));
        }
    }
    if cfg!(windows) {
        if let Some(local) = env_path("LOCALAPPDATA") {
            candidates.push(local.join("Android").join("Sdk"));
        }
    }
    candidates
}

fn looks_like_sdk(path: &Path) -> bool {
    path.join("platform-tools").is_dir() || path.join("emulator").is_dir()
}

/// Searches `PATH` plus a few well-known locations GUI apps usually miss.
pub fn which(name: &str) -> Option<PathBuf> {
    let file = format!("{name}{EXE_SUFFIX}");
    let mut dirs: Vec<PathBuf> = env::var_os("PATH")
        .map(|p| env::split_paths(&p).collect())
        .unwrap_or_default();
    if cfg!(unix) {
        dirs.extend(["/opt/homebrew/bin", "/usr/local/bin"].iter().map(PathBuf::from));
    }
    dirs.into_iter().map(|d| d.join(&file)).find(|p| p.is_file())
}

pub fn resolve(sdk_override: Option<&Path>) -> Tools {
    let mut android_sdk = sdk_candidates(sdk_override).into_iter().find(|p| looks_like_sdk(p));

    let sdk_tool = |sdk: &Option<PathBuf>, dir: &str, name: &str| {
        sdk.as_ref()
            .map(|s| s.join(dir).join(format!("{name}{EXE_SUFFIX}")))
            .filter(|p| p.is_file())
    };

    let adb = sdk_tool(&android_sdk, "platform-tools", "adb").or_else(|| which("adb"));

    // If adb came from PATH, its real location usually tells us where the SDK is.
    if android_sdk.is_none() {
        android_sdk = adb
            .as_ref()
            .and_then(|a| a.canonicalize().ok())
            .and_then(|a| a.parent()?.parent().map(Path::to_path_buf))
            .filter(|p| looks_like_sdk(p));
    }

    let emulator = sdk_tool(&android_sdk, "emulator", "emulator").or_else(|| which("emulator"));

    let xcrun = if cfg!(target_os = "macos") {
        Some(PathBuf::from("/usr/bin/xcrun")).filter(|p| p.is_file())
    } else {
        None
    };

    Tools { android_sdk, adb, emulator, xcrun }
}
