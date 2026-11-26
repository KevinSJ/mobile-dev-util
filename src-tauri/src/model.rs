use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Android,
    Ios,
}

impl Platform {
    pub fn key(self) -> &'static str {
        match self {
            Platform::Android => "android",
            Platform::Ios => "ios",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceState {
    Shutdown,
    Booting,
    Booted,
    ShuttingDown,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    /// AVD name (Android) or UDID (iOS).
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub state: DeviceState,
    /// e.g. "API 34" or "iOS 17.2".
    pub runtime: Option<String>,
    /// Extra info such as the system image flavour / ABI.
    pub detail: Option<String>,
    /// adb serial (e.g. `emulator-5554`) while an Android emulator is running.
    pub serial: Option<String>,
    pub recording: bool,
}

/// Identifies the device a command should act on.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceTarget {
    pub platform: Platform,
    pub id: String,
    pub name: String,
    pub serial: Option<String>,
}

impl DeviceTarget {
    pub fn key(&self) -> String {
        format!("{}:{}", self.platform.key(), self.id)
    }

    pub fn serial(&self) -> Result<&str, String> {
        self.serial
            .as_deref()
            .ok_or_else(|| format!("{} is not running. Boot it first.", self.name))
    }
}

/// Turns a device name into something safe to use in a file name.
pub fn file_stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    let stem = stem.trim_matches('_');
    if stem.is_empty() { "device".to_string() } else { stem.to_string() }
}
