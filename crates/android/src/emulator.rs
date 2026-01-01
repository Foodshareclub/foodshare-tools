//! Android Emulator management
//!
//! Provides tools for managing Android emulators.

use foodshare_core::error::Result;
use foodshare_core::process::{command_exists, run_command, CommandResult};
use serde::{Deserialize, Serialize};

/// Emulator device info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorDevice {
    pub name: String,
    pub status: EmulatorStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EmulatorStatus {
    Running,
    Stopped,
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

/// List available AVDs (Android Virtual Devices)
pub fn list_avds() -> Result<Vec<String>> {
    let result = run_command("emulator", &["-list-avds"])?;
    Ok(result
        .stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

/// List running emulators
pub fn list_running() -> Result<Vec<String>> {
    let result = run_command("adb", &["devices"])?;
    Ok(result
        .stdout
        .lines()
        .skip(1) // Skip header
        .filter(|l| l.contains("emulator"))
        .filter_map(|l| l.split_whitespace().next())
        .map(String::from)
        .collect())
}

/// Boot an emulator by AVD name
pub fn boot(avd_name: &str) -> Result<CommandResult> {
    // Start emulator in background
    run_command("emulator", &["-avd", avd_name, "-no-snapshot-load"])
}

/// Shutdown an emulator
pub fn shutdown(serial: &str) -> Result<CommandResult> {
    run_command("adb", &["-s", serial, "emu", "kill"])
}

/// Shutdown all emulators
pub fn shutdown_all() -> Result<()> {
    let running = list_running()?;
    for serial in running {
        let _ = shutdown(&serial);
    }
    Ok(())
}

/// Install an APK on an emulator
pub fn install_apk(serial: &str, apk_path: &str) -> Result<CommandResult> {
    run_command("adb", &["-s", serial, "install", "-r", apk_path])
}

/// Launch an app on an emulator
pub fn launch_app(serial: &str, package: &str, activity: &str) -> Result<CommandResult> {
    let component = format!("{}/{}", package, activity);
    run_command(
        "adb",
        &["-s", serial, "shell", "am", "start", "-n", &component],
    )
}

/// Get logcat output
pub fn logcat(serial: &str, filter: Option<&str>) -> Result<CommandResult> {
    let mut args = vec!["-s", serial, "logcat", "-d"];
    if let Some(f) = filter {
        args.push("-s");
        args.push(f);
    }
    run_command("adb", &args)
}

/// Clear logcat
pub fn clear_logcat(serial: &str) -> Result<CommandResult> {
    run_command("adb", &["-s", serial, "logcat", "-c"])
}

/// Take a screenshot
pub fn screenshot(serial: &str, output_path: &str) -> Result<CommandResult> {
    // Take screenshot on device
    let device_path = "/sdcard/screenshot.png";
    run_command(
        "adb",
        &["-s", serial, "shell", "screencap", "-p", device_path],
    )?;

    // Pull to local
    run_command("adb", &["-s", serial, "pull", device_path, output_path])
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
