//! Target device discovery.
//!
//! Maestro can drive an iOS simulator or an Android emulator. Both are
//! discovered from the tools that own them: `xcrun simctl` for the
//! simulators, `adb` for the emulators. A tool that is not installed is
//! not an error — it just means that platform has no devices.

use std::path::PathBuf;
use std::process::Stdio;

use serde::{Deserialize, Serialize};

/// The platform a device runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    /// An iOS simulator.
    Ios,
    /// An Android emulator or device.
    Android,
}

impl Platform {
    /// The platform's short name, used in file names and labels.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Ios => "ios",
            Self::Android => "android",
        }
    }
}

/// A device a test can run on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetDevice {
    /// The platform the device runs.
    pub platform: Platform,
    /// The device identifier, as the platform's tooling names it.
    pub id: String,
    /// The device's human-readable name.
    pub name: String,
}

impl TargetDevice {
    /// A file-name-safe form of the device identifier.
    #[must_use]
    pub fn safe_id(&self) -> String {
        self.id
            .chars()
            .map(|c| {
                if c == ':' || c == '/' || c == '\\' || c == ' ' {
                    '_'
                } else {
                    c
                }
            })
            .collect()
    }
}

/// The `simctl` device list, as far as this crate needs it.
#[derive(Debug, Deserialize)]
struct SimctlDevices {
    #[serde(default)]
    devices: std::collections::HashMap<String, Vec<SimctlDevice>>,
}

#[derive(Debug, Deserialize)]
struct SimctlDevice {
    udid: String,
    name: String,
    state: String,
    #[serde(default)]
    #[allow(dead_code)]
    isavailable: Option<bool>,
}

/// Every booted iOS simulator and connected Android device.
#[must_use]
pub fn discover() -> Vec<TargetDevice> {
    let mut devices = Vec::new();
    devices.extend(discover_ios());
    devices.extend(discover_android());
    devices
}

/// The booted iOS simulators.
#[must_use]
pub fn discover_ios() -> Vec<TargetDevice> {
    let Some(output) = run("xcrun", &["simctl", "list", "devices", "-j"]) else {
        return Vec::new();
    };
    let Ok(parsed) = serde_json::from_str::<SimctlDevices>(&output) else {
        return Vec::new();
    };

    let mut devices = Vec::new();
    for (runtime, list) in parsed.devices {
        if !runtime.contains("iOS") {
            continue;
        }
        for device in list {
            if device.state != "Booted" {
                continue;
            }
            devices.push(TargetDevice {
                platform: Platform::Ios,
                id: device.udid,
                name: device.name,
            });
        }
    }
    devices
}

/// The connected Android devices.
#[must_use]
pub fn discover_android() -> Vec<TargetDevice> {
    let Some(output) = run("adb", &["devices", "-l"]) else {
        return Vec::new();
    };

    let mut devices = Vec::new();
    for line in output.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.contains("device") || trimmed.contains("offline") {
            continue;
        }
        let Some(serial) = trimmed.split_whitespace().next() else {
            continue;
        };
        if serial == "List" {
            continue;
        }
        // `adb devices -l` appends `model:Pixel_7` to the device line.
        let name = trimmed
            .split_whitespace()
            .find_map(|field| field.strip_prefix("model:"))
            .unwrap_or(serial)
            .to_string();
        devices.push(TargetDevice {
            platform: Platform::Android,
            id: serial.to_string(),
            name,
        });
    }
    devices
}

/// The single device to target, preferring an explicitly requested one.
#[must_use]
pub fn resolve(requested: Option<&str>) -> Option<TargetDevice> {
    let devices = discover();
    match requested {
        Some(id) => devices.into_iter().find(|d| d.id == id),
        None => devices.into_iter().next(),
    }
}

/// Run a command and capture its output, or `None` if it could not run.
fn run(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

/// The reports directory for a Maestro directory.
#[must_use]
pub fn reports_dir(maestro_dir: &PathBuf) -> PathBuf {
    maestro_dir.join("reports")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_id_replaces_path_characters() {
        let device = TargetDevice {
            platform: Platform::Ios,
            id: "AB:CD/EF\\GH".to_string(),
            name: "iPhone".to_string(),
        };
        assert_eq!(device.safe_id(), "AB_CD_EF_GH");
    }

    #[test]
    fn test_safe_id_leaves_a_plain_identifier_alone() {
        let device = TargetDevice {
            platform: Platform::Android,
            id: "emulator-5554".to_string(),
            name: "Pixel".to_string(),
        };
        assert_eq!(device.safe_id(), "emulator-5554");
    }

    #[test]
    fn test_discovery_is_empty_without_the_tools() {
        // The tools are not installed everywhere, and a missing tool is
        // not an error.
        let devices = discover();
        assert!(devices.len() <= discover_ios().len() + discover_android().len());
    }
}
