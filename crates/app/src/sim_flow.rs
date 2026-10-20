//! The simulator flow.
//!
//! Boots a simulator, builds the app for it, installs it, launches it,
//! and takes a screenshot. The flow is the one the Bun runner used; the
//! pieces live in [`crate::build`] and [`crate::simulator`].

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use foodshare_core::error::{Error, Result};
use serde::{Deserialize, Serialize};

use crate::build::{self, Configuration, Layout};

/// The bundle identifier of the app.
pub const BUNDLE_ID: &str = "com.flutterflow.foodshare";

/// How long the UI is given to settle before the screenshot.
const SETTLE: Duration = Duration::from_millis(1500);

/// Whether a simulator is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SimulatorState {
    /// The simulator is running.
    Booted,
    /// The simulator is shut down.
    Shutdown,
}

/// A simulator device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulatorDevice {
    /// The device identifier.
    pub id: String,
    /// The device's name.
    pub name: String,
    /// Whether it is running.
    pub state: SimulatorState,
}

/// The simulators on this machine, booted and available.
#[must_use]
pub fn available_simulators() -> (Vec<SimulatorDevice>, Vec<SimulatorDevice>) {
    let mut booted = Vec::new();
    let mut available = Vec::new();

    for device in crate::simulator::list_devices().unwrap_or_default() {
        let state = if device.state == "Booted" {
            SimulatorState::Booted
        } else {
            SimulatorState::Shutdown
        };
        let info = SimulatorDevice {
            id: device.udid,
            name: device.name,
            state,
        };
        if state == SimulatorState::Booted {
            booted.push(info);
        } else {
            available.push(info);
        }
    }

    (booted, available)
}

/// The simulator to run on, booting one if needed.
///
/// A booted simulator is preferred. Otherwise the newest iPhone Pro is
/// chosen, and booted.
pub fn resolve_or_boot(requested: Option<&str>) -> Result<SimulatorDevice> {
    let (booted, available) = available_simulators();

    if let Some(id) = requested {
        let target = booted
            .iter()
            .chain(available.iter())
            .find(|d| d.id == id)
            .cloned()
            .ok_or_else(|| {
                Error::validation(format!("Requested simulator device ID {id} not found."))
            })?;
        if target.state == SimulatorState::Shutdown {
            boot(&target.id)?;
        }
        return Ok(SimulatorDevice {
            state: SimulatorState::Booted,
            ..target
        });
    }

    if let Some(device) = booted.into_iter().next() {
        return Ok(device);
    }

    let preferred = available
        .iter()
        .find(|d| d.name.contains("iPhone 17 Pro"))
        .or_else(|| available.iter().find(|d| d.name.contains("iPhone 16 Pro")))
        .or_else(|| available.iter().find(|d| d.name.contains("iPhone 15 Pro")))
        .or_else(|| available.iter().find(|d| d.name.starts_with("iPhone")))
        .or_else(|| available.first())
        .cloned()
        .ok_or_else(|| Error::validation("No compatible iOS Simulator found on this system."))?;

    boot(&preferred.id)?;
    Ok(SimulatorDevice {
        state: SimulatorState::Booted,
        ..preferred
    })
}

/// Boot a simulator.
pub fn boot(device_id: &str) -> Result<()> {
    crate::simulator::boot(device_id).map(|_| ())
}

/// Install the app on a simulator and launch it.
///
/// Returns the process identifier of the launched app, or `None` when
/// the launch did not report one.
pub fn install_and_launch(device: &SimulatorDevice, app_path: &Path) -> Result<Option<u64>> {
    crate::simulator::install_app(&device.id, &app_path.display().to_string())
        .map_err(|e| Error::process(format!("Installation failed: {e}")))?;

    // A previous instance would hold the app open.
    let _ = std::process::Command::new("xcrun")
        .args(["simctl", "terminate", &device.id, BUNDLE_ID])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let output = std::process::Command::new("xcrun")
        .args(["simctl", "launch", &device.id, BUNDLE_ID])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| Error::process(format!("Launch failed: {e}")))?;

    let text = String::from_utf8_lossy(&output.stdout).to_string();
    // The launch output ends with the process identifier: `com.x: 12345`.
    let pid = text
        .rsplit(": ")
        .next()
        .and_then(|part| part.trim().parse::<u64>().ok());
    Ok(pid)
}

/// Take a screenshot of the simulator.
pub fn capture_screenshot(
    device: &SimulatorDevice,
    app_dir: &Path,
    out_path: Option<&Path>,
) -> Result<PathBuf> {
    let target = out_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| app_dir.join(".build").join("simulator_screenshot.png"));

    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // The UI is given a moment to settle first, so the screenshot is of
    // the app rather than of a blank screen.
    std::thread::sleep(SETTLE);

    crate::simulator::screenshot(&device.id, &target.display().to_string())
        .map_err(|e| Error::process(format!("Screenshot failed: {e}")))?;
    Ok(target)
}

/// Build the app for a simulator and return the bundle.
pub async fn build_for_device(
    layout: &Layout,
    device: &SimulatorDevice,
    clean: bool,
) -> Result<PathBuf> {
    let destination = format!("id={}", device.id);
    let outcome = build::build_ios(layout, Configuration::Debug, clean, &destination).await;
    if !outcome.success {
        return Err(Error::process(
            outcome
                .error
                .unwrap_or_else(|| "the iOS build failed".to_string()),
        ));
    }
    let bundle = layout.app_bundle(Configuration::Debug);
    if !bundle.is_dir() {
        return Err(Error::file_not_found(&bundle));
    }
    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_simulators_returns_two_lists() {
        let (booted, available) = available_simulators();
        // The lists partition the simulators, so no device is in both.
        for device in &booted {
            assert_eq!(device.state, SimulatorState::Booted);
            assert!(!available.iter().any(|d| d.id == device.id));
        }
    }

    #[test]
    fn test_resolve_or_boot_rejects_an_unknown_device() {
        let error = resolve_or_boot(Some("no-such-device")).expect_err("unknown device");
        assert!(error.to_string().contains("not found"));
    }
}
