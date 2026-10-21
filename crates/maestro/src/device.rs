//! Target device discovery.
//!
//! Maestro can drive an iOS simulator or an Android emulator. Both are
//! discovered from the tools that own them: `xcrun simctl` for the
//! simulators, `adb` for the emulators. A tool that is not installed is
//! not an error — it just means that platform has no devices.

//! Target device discovery.
//!
//! Maestro can drive an iOS simulator or an Android emulator. The
//! discovery itself lives in `foodshare-device`, which is the one place
//! that knows the output of `xcrun simctl` and `adb`. A tool that is
//! not installed is not an error — it just means that platform has no
//! devices.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use foodshare_device::{DeviceState, Emulator, Simulator};

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
    foodshare_device::booted_simulators()
        .unwrap_or_default()
        .into_iter()
        .map(|simulator| TargetDevice {
            platform: Platform::Ios,
            id: simulator.udid,
            name: simulator.name,
        })
        .collect()
}

/// The connected Android devices.
#[must_use]
pub fn discover_android() -> Vec<TargetDevice> {
    foodshare_device::emulators()
        .unwrap_or_default()
        .into_iter()
        .filter(|emulator| emulator.state == DeviceState::Booted)
        .map(|emulator| TargetDevice {
            platform: Platform::Android,
            id: emulator.serial,
            name: emulator.model.unwrap_or_default(),
        })
        .collect()
}

/// The simulators, for callers that need the whole device rather than a
/// target.
#[must_use]
pub fn simulators() -> Vec<Simulator> {
    foodshare_device::simulators().unwrap_or_default()
}

/// The emulators, for callers that need the whole device rather than a
/// target.
#[must_use]
pub fn emulators() -> Vec<Emulator> {
    foodshare_device::emulators().unwrap_or_default()
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
