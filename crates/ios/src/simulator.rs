//! iOS Simulator management
//!
//! Provides tools for managing iOS simulators.

use foodshare_core::error::Result;
use foodshare_core::process::{run_command, CommandResult};
use serde::{Deserialize, Serialize};

/// Simulator device info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatorDevice {
    pub udid: String,
    pub name: String,
    pub state: String,
    pub runtime: String,
    pub is_available: bool,
}

/// List available simulators
pub fn list_devices() -> Result<Vec<SimulatorDevice>> {
    let result = run_command("xcrun", &["simctl", "list", "devices", "-j"])?;

    let json: serde_json::Value = serde_json::from_str(&result.stdout)?;
    let mut devices = Vec::new();

    if let Some(device_map) = json["devices"].as_object() {
        for (runtime, runtime_devices) in device_map {
            if let Some(arr) = runtime_devices.as_array() {
                for device in arr {
                    if let (Some(udid), Some(name), Some(state)) = (
                        device["udid"].as_str(),
                        device["name"].as_str(),
                        device["state"].as_str(),
                    ) {
                        let is_available = device["isAvailable"].as_bool().unwrap_or(false);
                        devices.push(SimulatorDevice {
                            udid: udid.to_string(),
                            name: name.to_string(),
                            state: state.to_string(),
                            runtime: runtime.clone(),
                            is_available,
                        });
                    }
                }
            }
        }
    }

    Ok(devices)
}

/// Boot a simulator by name or UDID
pub fn boot(device: &str) -> Result<CommandResult> {
    run_command("xcrun", &["simctl", "boot", device])
}

/// Shutdown a simulator
pub fn shutdown(device: &str) -> Result<CommandResult> {
    run_command("xcrun", &["simctl", "shutdown", device])
}

/// Shutdown all simulators
pub fn shutdown_all() -> Result<CommandResult> {
    run_command("xcrun", &["simctl", "shutdown", "all"])
}

/// Erase a simulator (reset to clean state)
pub fn erase(device: &str) -> Result<CommandResult> {
    run_command("xcrun", &["simctl", "erase", device])
}

/// Install an app on a simulator
pub fn install_app(device: &str, app_path: &str) -> Result<CommandResult> {
    run_command("xcrun", &["simctl", "install", device, app_path])
}

/// Launch an app on a simulator
pub fn launch_app(device: &str, bundle_id: &str) -> Result<CommandResult> {
    run_command("xcrun", &["simctl", "launch", device, bundle_id])
}

/// Take a screenshot
pub fn screenshot(device: &str, output_path: &str) -> Result<CommandResult> {
    run_command(
        "xcrun",
        &["simctl", "io", device, "screenshot", output_path],
    )
}

/// Open Simulator app
pub fn open_simulator() -> Result<CommandResult> {
    run_command("open", &["-a", "Simulator"])
}

/// Get booted device UDID
pub fn get_booted_device() -> Result<Option<String>> {
    let devices = list_devices()?;
    Ok(devices
        .into_iter()
        .find(|d| d.state == "Booted")
        .map(|d| d.udid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_device_struct() {
        let device = SimulatorDevice {
            udid: "test-udid".to_string(),
            name: "iPhone 15 Pro".to_string(),
            state: "Shutdown".to_string(),
            runtime: "iOS 17.0".to_string(),
            is_available: true,
        };
        assert_eq!(device.name, "iPhone 15 Pro");
    }
}
