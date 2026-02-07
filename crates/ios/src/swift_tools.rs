//! Swift tooling wrappers
//!
//! Provides wrappers for Swift development tools.

use foodshare_core::error::Result;
use foodshare_core::process::{command_exists, run_command, run_command_in_dir, CommandResult};
use std::path::Path;

/// Check if swiftformat is available
pub fn has_swiftformat() -> bool {
    command_exists("swiftformat")
}

/// Check if swiftlint is available
pub fn has_swiftlint() -> bool {
    command_exists("swiftlint")
}

/// Format Swift files with swiftformat
pub fn format(files: &[&str], check_only: bool) -> Result<CommandResult> {
    let mut args: Vec<&str> = files.to_vec();

    if check_only {
        args.push("--lint");
    }

    run_command("swiftformat", &args)
}

/// Format Swift files in a directory
pub fn format_directory(dir: &Path, check_only: bool) -> Result<CommandResult> {
    let mut args = vec![dir.to_string_lossy().to_string()];

    if check_only {
        args.push("--lint".to_string());
    }

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_command("swiftformat", &args_refs)
}

/// Lint Swift files with swiftlint
pub fn lint(files: &[&str], strict: bool, fix: bool) -> Result<CommandResult> {
    let mut args = vec!["lint"];

    if strict {
        args.push("--strict");
    }

    if fix {
        args = vec!["--fix"];
    }

    for file in files {
        args.push(file);
    }

    run_command("swiftlint", &args)
}

/// Lint all Swift files in a directory
pub fn lint_directory(dir: &Path, strict: bool, fix: bool) -> Result<CommandResult> {
    let mut args = if fix {
        vec!["--fix"]
    } else {
        vec!["lint"]
    };

    if strict && !fix {
        args.push("--strict");
    }

    run_command_in_dir("swiftlint", &args, dir)
}

/// Build Swift package
pub fn build_package(package_dir: &Path, configuration: &str) -> Result<CommandResult> {
    run_command_in_dir(
        "swift",
        &["build", "-c", configuration],
        package_dir,
    )
}

/// Test Swift package
pub fn test_package(package_dir: &Path, filter: Option<&str>) -> Result<CommandResult> {
    let mut args = vec!["test"];

    if let Some(f) = filter {
        args.push("--filter");
        args.push(f);
    }

    run_command_in_dir("swift", &args, package_dir)
}

/// Resolve Swift package dependencies
pub fn resolve_dependencies(package_dir: &Path) -> Result<CommandResult> {
    run_command_in_dir("swift", &["package", "resolve"], package_dir)
}

/// Update Swift package dependencies
pub fn update_dependencies(package_dir: &Path) -> Result<CommandResult> {
    run_command_in_dir("swift", &["package", "update"], package_dir)
}

/// Get Swift version
pub fn swift_version() -> Result<String> {
    let result = run_command("swift", &["--version"])?;
    Ok(result.stdout.lines().next().unwrap_or("Unknown").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_swiftformat() {
        // Just verify it doesn't panic
        let _ = has_swiftformat();
    }

    #[test]
    fn test_has_swiftlint() {
        let _ = has_swiftlint();
    }
}
