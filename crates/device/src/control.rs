//! Controlling the devices.
//!
//! The discovery lives in the crate's other module; this one drives the
//! devices. Both know what `xcrun simctl` and `adb` say, so both are
//! here: one crate owns the devices.

use foodshare_core::error::Result;
use foodshare_core::process::{CommandResult, run_command};

use crate::{DeviceState, Emulator, Simulator};

/// Boot a simulator.
pub fn boot_simulator(device: &str) -> Result<CommandResult> {
    run_command("xcrun", &["simctl", "boot", device])
}

/// Shut a simulator down.
pub fn shutdown_simulator(device: &str) -> Result<CommandResult> {
    run_command("xcrun", &["simctl", "shutdown", device])
}

/// Shut every simulator down.
pub fn shutdown_all_simulators() -> Result<CommandResult> {
    run_command("xcrun", &["simctl", "shutdown", "all"])
}

/// Erase a simulator, back to a clean state.
pub fn erase_simulator(device: &str) -> Result<CommandResult> {
    run_command("xcrun", &["simctl", "erase", device])
}

/// Install an app on a simulator.
pub fn install_app(device: &str, app_path: &str) -> Result<CommandResult> {
    run_command("xcrun", &["simctl", "install", device, app_path])
}

/// Launch an app on a simulator.
pub fn launch_app(device: &str, bundle_id: &str) -> Result<CommandResult> {
    run_command("xcrun", &["simctl", "launch", device, bundle_id])
}

/// Take a screenshot of a simulator.
pub fn screenshot(device: &str, output_path: &str) -> Result<CommandResult> {
    run_command(
        "xcrun",
        &["simctl", "io", device, "screenshot", output_path],
    )
}

/// Open the Simulator application.
pub fn open_simulator() -> Result<CommandResult> {
    run_command("open", &["-a", "Simulator"])
}

/// The booted simulator, if there is one.
pub fn booted_simulator() -> Result<Option<Simulator>> {
    Ok(crate::simulators()?
        .into_iter()
        .find(|s| s.state == DeviceState::Booted))
}

/// The identifier of the booted simulator, if there is one.
pub fn booted_simulator_id() -> Result<Option<String>> {
    Ok(booted_simulator()?.map(|s| s.udid))
}

/// The available Android virtual devices.
pub fn list_avds() -> Result<Vec<String>> {
    let result = run_command("emulator", &["-list-avds"])?;
    Ok(result
        .stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

/// The running Android devices.
pub fn list_running_emulators() -> Result<Vec<Emulator>> {
    Ok(crate::emulators()?
        .into_iter()
        .filter(|e| e.state == DeviceState::Booted)
        .collect())
}

/// Boot an Android virtual device.
pub fn boot_emulator(avd_name: &str) -> Result<CommandResult> {
    run_command("emulator", &["-avd", avd_name, "-no-snapshot-load"])
}

/// Shut an Android device down.
pub fn shutdown_emulator(serial: &str) -> Result<CommandResult> {
    run_command("adb", &["-s", serial, "emu", "kill"])
}

/// Shut every Android device down.
pub fn shutdown_all_emulators() -> Result<CommandResult> {
    for emulator in list_running_emulators()? {
        let _ = shutdown_emulator(&emulator.serial);
    }
    Ok(CommandResult {
        success: true,
        exit_code: 0,
        stdout: String::new(),
        stderr: String::new(),
    })
}

/// Install an APK on an Android device.
pub fn install_apk(serial: &str, apk_path: &str) -> Result<CommandResult> {
    run_command("adb", &["-s", serial, "install", "-r", apk_path])
}

/// Launch an app on an Android device.
pub fn launch_android_app(serial: &str, package: &str, activity: &str) -> Result<CommandResult> {
    let component = format!("{package}/{activity}");
    run_command(
        "adb",
        &["-s", serial, "shell", "am", "start", "-n", &component],
    )
}

/// The device log.
pub fn logcat(serial: &str, filter: Option<&str>) -> Result<CommandResult> {
    let mut args = vec!["-s", serial, "logcat", "-d"];
    if let Some(filter) = filter {
        args.push("-s");
        args.push(filter);
    }
    run_command("adb", &args)
}

/// Clear the device log.
pub fn clear_logcat(serial: &str) -> Result<CommandResult> {
    run_command("adb", &["-s", serial, "logcat", "-c"])
}

/// Take a screenshot of an Android device.
pub fn screenshot_android(serial: &str, output_path: &str) -> Result<CommandResult> {
    let device_path = "/sdcard/screenshot.png";
    run_command(
        "adb",
        &["-s", serial, "shell", "screencap", "-p", device_path],
    )?;
    run_command("adb", &["-s", serial, "pull", device_path, output_path])
}
