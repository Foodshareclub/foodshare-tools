//! Xcode project utilities
//!
//! Provides tools for working with Xcode projects and workspaces.

use foodshare_core::error::Result;
use foodshare_core::process::{
    CommandResult, command_exists, run_command, run_command_with_timeout,
};
use std::path::Path;
use std::time::Duration;

/// Check if xcodebuild is available
pub fn is_xcode_available() -> bool {
    command_exists("xcodebuild")
}

/// Get Xcode version
pub fn xcode_version() -> Result<String> {
    let result = run_command("xcodebuild", &["-version"])?;
    Ok(result
        .stdout
        .lines()
        .next()
        .unwrap_or("Unknown")
        .to_string())
}

/// Build an Xcode project
pub fn build(
    scheme: &str,
    configuration: &str,
    destination: &str,
    clean: bool,
) -> Result<CommandResult> {
    build_with_timeout(scheme, configuration, destination, clean, None)
}

/// Build an Xcode project, giving up after `timeout`.
///
/// A build that hangs is a worse failure than a build that fails: the
/// caller waits forever and learns nothing.
pub fn build_with_timeout(
    scheme: &str,
    configuration: &str,
    destination: &str,
    clean: bool,
    timeout: Option<Duration>,
) -> Result<CommandResult> {
    let mut args = vec![
        "-scheme",
        scheme,
        "-configuration",
        configuration,
        "-destination",
        destination,
    ];

    if clean {
        args.push("clean");
    }
    args.push("build");

    run_with_timeout("xcodebuild", &args, timeout)
}

/// Run tests for an Xcode project
pub fn test(scheme: &str, destination: &str, coverage: bool) -> Result<CommandResult> {
    test_with_timeout(scheme, destination, coverage, None)
}

/// Run tests for an Xcode project, giving up after `timeout`.
pub fn test_with_timeout(
    scheme: &str,
    destination: &str,
    coverage: bool,
    timeout: Option<Duration>,
) -> Result<CommandResult> {
    let mut args = vec!["-scheme", scheme, "-destination", destination, "test"];

    if coverage {
        args.push("-enableCodeCoverage");
        args.push("YES");
    }

    run_with_timeout("xcodebuild", &args, timeout)
}

/// Run a command, with a deadline when one is given.
fn run_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Option<Duration>,
) -> Result<CommandResult> {
    match timeout {
        Some(timeout) => run_command_with_timeout(program, args, timeout),
        None => run_command(program, args),
    }
}

/// Archive an Xcode project
pub fn archive(scheme: &str, archive_path: &Path) -> Result<CommandResult> {
    run_command(
        "xcodebuild",
        &[
            "-scheme",
            scheme,
            "-archivePath",
            &archive_path.to_string_lossy(),
            "archive",
        ],
    )
}

/// Get list of available schemes
pub fn list_schemes(project_path: &Path) -> Result<Vec<String>> {
    let result = run_command(
        "xcodebuild",
        &[
            "-project",
            &project_path.to_string_lossy(),
            "-list",
            "-json",
        ],
    )?;

    // Parse JSON output to extract schemes
    let json: serde_json::Value = serde_json::from_str(&result.stdout)?;
    let schemes = json["project"]["schemes"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(schemes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_xcode_available() {
        // This will be true on macOS with Xcode installed
        let _ = is_xcode_available();
    }
}
