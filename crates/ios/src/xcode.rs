//! Xcode project utilities
//!
//! Provides tools for working with Xcode projects and workspaces.

use foodshare_core::error::Result;
use foodshare_core::process::{command_exists, run_command, CommandResult};
use std::path::Path;

/// Check if xcodebuild is available
pub fn is_xcode_available() -> bool {
    command_exists("xcodebuild")
}

/// Get Xcode version
pub fn xcode_version() -> Result<String> {
    let result = run_command("xcodebuild", &["-version"])?;
    Ok(result.stdout.lines().next().unwrap_or("Unknown").to_string())
}

/// Build an Xcode project
pub fn build(
    scheme: &str,
    configuration: &str,
    destination: &str,
    clean: bool,
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

    run_command("xcodebuild", &args)
}

/// Run tests for an Xcode project
pub fn test(scheme: &str, destination: &str, coverage: bool) -> Result<CommandResult> {
    let mut args = vec!["-scheme", scheme, "-destination", destination, "test"];

    if coverage {
        args.push("-enableCodeCoverage");
        args.push("YES");
    }

    run_command("xcodebuild", &args)
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
        &["-project", &project_path.to_string_lossy(), "-list", "-json"],
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
