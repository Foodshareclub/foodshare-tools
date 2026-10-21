//! Device discovery: the iOS simulators and the Android emulators.
//!
//! The two tools that own the devices — `xcrun simctl` and `adb` —
//! report them in different shapes, and this crate is the one place that
//! knows both. The app tools drive a device; the end-to-end runner picks
//! one to run on. Neither of them should be parsing the output.
//!
//! A tool that is not installed is not an error: it means that platform
//! has no devices.

use foodshare_core::error::Result;
use foodshare_core::process::run_command;
use serde::{Deserialize, Serialize};

/// Whether a device is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceState {
    /// The device is running.
    Booted,
    /// The device is shut down.
    Shutdown,
    /// The device is connected but not usable.
    Offline,
}

impl DeviceState {
    /// Read the state from the tool's own wording.
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value {
            "Booted" | "device" => Self::Booted,
            "offline" => Self::Offline,
            _ => Self::Shutdown,
        }
    }
}

/// An iOS simulator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Simulator {
    /// The device identifier.
    pub udid: String,
    /// The device's name.
    pub name: String,
    /// The iOS runtime it runs.
    pub runtime: String,
    /// Whether it is running.
    pub state: DeviceState,
    /// Whether the runtime is installed and the device can be booted.
    pub is_available: bool,
}

/// An Android emulator or device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Emulator {
    /// The device's serial.
    pub serial: String,
    /// The device model, when `adb` reports one.
    pub model: Option<String>,
    /// Whether it is connected.
    pub state: DeviceState,
}

/// The simulators on this machine.
pub fn simulators() -> Result<Vec<Simulator>> {
    let output = run_command("xcrun", &["simctl", "list", "devices", "-j"])?;
    parse_simulators(&output.stdout)
}

/// The simulators that are running.
pub fn booted_simulators() -> Result<Vec<Simulator>> {
    Ok(simulators()?
        .into_iter()
        .filter(|s| s.state == DeviceState::Booted)
        .collect())
}

/// The connected Android devices.
pub fn emulators() -> Result<Vec<Emulator>> {
    let output = run_command("adb", &["devices", "-l"])?;
    Ok(parse_emulators(&output.stdout))
}

/// The simulators `simctl` reports.
fn parse_simulators(output: &str) -> Result<Vec<Simulator>> {
    #[derive(Deserialize)]
    struct Listing {
        #[serde(default)]
        devices: std::collections::HashMap<String, Vec<Entry>>,
    }

    #[derive(Deserialize)]
    struct Entry {
        udid: String,
        name: String,
        #[serde(default)]
        state: String,
        #[serde(default, rename = "isAvailable")]
        is_available: Option<bool>,
    }

    let listing: Listing = serde_json::from_str(output).map_err(|e| {
        foodshare_core::error::Error::process(format!("could not read the simulator list: {e}"))
    })?;

    let mut simulators = Vec::new();
    for (runtime, entries) in listing.devices {
        if !runtime.contains("iOS") {
            continue;
        }
        for entry in entries {
            simulators.push(Simulator {
                udid: entry.udid,
                name: entry.name,
                runtime: runtime.clone(),
                state: DeviceState::parse(&entry.state),
                is_available: entry.is_available.unwrap_or(true),
            });
        }
    }
    Ok(simulators)
}

/// The devices `adb devices -l` reports.
fn parse_emulators(output: &str) -> Vec<Emulator> {
    let mut emulators = Vec::new();
    for line in output.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(serial) = trimmed.split_whitespace().next() else {
            continue;
        };
        if serial == "List" {
            continue;
        }
        if !trimmed.contains("device") {
            // Offline, unauthorised, or in recovery.
            emulators.push(Emulator {
                serial: serial.to_string(),
                model: None,
                state: DeviceState::Offline,
            });
            continue;
        }
        let model = trimmed
            .split_whitespace()
            .find_map(|field| field.strip_prefix("model:"))
            .map(str::to_string);
        emulators.push(Emulator {
            serial: serial.to_string(),
            model,
            state: DeviceState::Booted,
        });
    }
    emulators
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parses_the_simulator_listing() {
        let output = r#"{
            "devices": {
                "com.apple.CoreSimulator.SimRuntime.iOS-18-0": [
                    {"udid": "AAA", "name": "iPhone 17 Pro", "state": "Booted", "isAvailable": true},
                    {"udid": "BBB", "name": "iPhone 16", "state": "Shutdown", "isAvailable": true}
                ],
                "com.apple.CoreSimulator.SimRuntime.watchOS-11-0": [
                    {"udid": "CCC", "name": "Watch", "state": "Booted", "isAvailable": true}
                ]
            }
        }"#;
        let simulators = parse_simulators(output).expect("parsed");
        assert_eq!(simulators.len(), 2);
        assert_eq!(simulators[0].udid, "AAA");
        assert_eq!(simulators[0].state, DeviceState::Booted);
        assert!(simulators[0].is_available);
        assert_eq!(simulators[1].state, DeviceState::Shutdown);
        assert!(simulators[0].runtime.contains("iOS-18-0"));
    }

    #[test]
    fn test_parses_the_emulator_listing() {
        let output = "List of devices attached\nemulator-5554\tdevice product:sdk_gphone64_arm64 model:Pixel_7 device:emu64a\n";
        let emulators = parse_emulators(output);
        assert_eq!(emulators.len(), 1);
        assert_eq!(emulators[0].serial, "emulator-5554");
        assert_eq!(emulators[0].model.as_deref(), Some("Pixel_7"));
        assert_eq!(emulators[0].state, DeviceState::Booted);
    }

    #[test]
    fn test_an_offline_device_is_reported_as_offline() {
        let output = "List of devices attached\nemulator-5556\toffline\n";
        let emulators = parse_emulators(output);
        assert_eq!(emulators.len(), 1);
        assert_eq!(emulators[0].state, DeviceState::Offline);
    }

    #[test]
    fn test_the_header_is_not_a_device() {
        let emulators = parse_emulators("List of devices attached\n");
        assert!(emulators.is_empty());
    }

    #[test]
    fn test_the_device_state_parses_the_tools_wording() {
        assert_eq!(DeviceState::parse("Booted"), DeviceState::Booted);
        assert_eq!(DeviceState::parse("Shutdown"), DeviceState::Shutdown);
        assert_eq!(DeviceState::parse("offline"), DeviceState::Offline);
        assert_eq!(DeviceState::parse("device"), DeviceState::Booted);
    }
}
