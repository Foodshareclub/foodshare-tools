//! Android Emulator management.
//!
//! The devices themselves — what `adb` reports and how to drive one —
//! live in `foodshare-device`. This module keeps the app tools' names for
//! them.

use foodshare_core::error::Result;
use foodshare_core::process::{CommandResult, command_exists};
use serde::{Deserialize, Serialize};

pub use foodshare_device::{
    boot_emulator, clear_logcat, install_apk, launch_android_app, list_avds,
    list_running_emulators, logcat, screenshot_android, shutdown_all_emulators, shutdown_emulator,
};

/// Android emulator device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorDevice {
    /// Name of the AVD (Android Virtual Device)
    pub name: String,
    /// Current execution status of the emulator
    pub status: EmulatorStatus,
}

/// Execution status of an Android emulator
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EmulatorStatus {
    /// Emulator is currently running
    Running,
    /// Emulator is stopped
    Stopped,
    /// Status could not be determined
    Unknown,
}

/// Check if emulator command is available
pub fn is_emulator_available() -> bool {
    command_exists("emulator")
}

/// Check if adb is available
pub fn is_adb_available() -> bool {
    command_exists("adb")
}

/// List running emulators
pub fn list_running() -> Result<Vec<String>> {
    Ok(list_running_emulators()?
        .into_iter()
        .map(|emulator| emulator.serial)
        .collect())
}

/// Boot an emulator by AVD name
pub fn boot(avd_name: &str) -> Result<CommandResult> {
    boot_emulator(avd_name)
}

/// Shutdown an emulator
pub fn shutdown(serial: &str) -> Result<CommandResult> {
    shutdown_emulator(serial)
}

/// Shutdown all emulators
pub fn shutdown_all() -> Result<CommandResult> {
    shutdown_all_emulators()
}

/// Launch an app on an emulator
pub fn launch_app(serial: &str, package: &str, activity: &str) -> Result<CommandResult> {
    launch_android_app(serial, package, activity)
}

/// Take a screenshot
pub fn screenshot(serial: &str, output_path: &str) -> Result<CommandResult> {
    foodshare_device::screenshot_android(serial, output_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emulator_status_enum() {
        assert_eq!(EmulatorStatus::Running, EmulatorStatus::Running);
        assert_ne!(EmulatorStatus::Running, EmulatorStatus::Stopped);
    }

    #[test]
    fn test_emulator_device_struct() {
        let device = EmulatorDevice {
            name: "Pixel_7_API_34".to_string(),
            status: EmulatorStatus::Stopped,
        };
        assert_eq!(device.name, "Pixel_7_API_34");
    }
}
