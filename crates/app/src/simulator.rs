//! iOS Simulator management.
//!
//! The devices themselves — what `xcrun simctl` reports and how to
//! drive one — live in `foodshare-device`. This module keeps the app
//! tools' names for them.

use foodshare_core::error::Result;
use foodshare_core::process::CommandResult;
use serde::{Deserialize, Serialize};

pub use foodshare_device::{
    boot_simulator, booted_simulator, booted_simulator_id, erase_simulator, install_app,
    launch_app, open_simulator, screenshot, shutdown_all_simulators, shutdown_simulator,
};

/// Information about an iOS simulator device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatorDevice {
    /// Unique device identifier
    pub udid: String,
    /// Human-readable name (e.g. "iPhone 15 Pro")
    pub name: String,
    /// Current device state (e.g. "Booted", "Shutdown")
    pub state: String,
    /// iOS runtime version (e.g. "iOS 17.0")
    pub runtime: String,
    /// Whether the device is available for use
    pub is_available: bool,
}

/// List available simulators
pub fn list_devices() -> Result<Vec<SimulatorDevice>> {
    Ok(foodshare_device::simulators()?
        .into_iter()
        .map(|simulator| SimulatorDevice {
            udid: simulator.udid,
            name: simulator.name,
            state: match simulator.state {
                foodshare_device::DeviceState::Booted => "Booted".to_string(),
                foodshare_device::DeviceState::Offline => "Offline".to_string(),
                foodshare_device::DeviceState::Shutdown => "Shutdown".to_string(),
            },
            runtime: simulator.runtime,
            is_available: simulator.is_available,
        })
        .collect())
}

/// Boot a simulator by name or UDID
pub fn boot(device: &str) -> Result<CommandResult> {
    boot_simulator(device)
}

/// Shutdown a simulator
pub fn shutdown(device: &str) -> Result<CommandResult> {
    shutdown_simulator(device)
}

/// Shutdown all simulators
pub fn shutdown_all() -> Result<CommandResult> {
    shutdown_all_simulators()
}

/// Erase a simulator (reset to clean state)
pub fn erase(device: &str) -> Result<CommandResult> {
    erase_simulator(device)
}

/// Get booted device UDID
pub fn get_booted_device() -> Result<Option<String>> {
    booted_simulator_id()
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
