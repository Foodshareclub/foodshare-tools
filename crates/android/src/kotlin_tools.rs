//! Kotlin tooling wrappers
//!
//! Provides wrappers for Kotlin development tools.

use foodshare_core::error::Result;
use foodshare_core::process::{command_exists, run_command, run_command_in_dir, CommandResult};
use std::path::Path;

/// Check if ktlint is available
pub fn has_ktlint() -> bool {
    command_exists("ktlint")
}

/// Check if detekt is available
pub fn has_detekt() -> bool {
    command_exists("detekt")
}

/// Format Kotlin files with ktlint
pub fn format(files: &[&str]) -> Result<CommandResult> {
    let mut args = vec!["-F"];
    args.extend(files);
    run_command("ktlint", &args)
}

/// Format Kotlin files in a directory
pub fn format_directory(dir: &Path) -> Result<CommandResult> {
    run_command_in_dir("ktlint", &["-F", "**/*.kt", "**/*.kts"], dir)
}

/// Check Kotlin files with ktlint (no fix)
pub fn check(files: &[&str]) -> Result<CommandResult> {
    run_command("ktlint", files)
}

/// Check Kotlin files in a directory
pub fn check_directory(dir: &Path) -> Result<CommandResult> {
    run_command_in_dir("ktlint", &["**/*.kt", "**/*.kts"], dir)
}

/// Run detekt static analysis
pub fn detekt_analyze(config_path: Option<&str>) -> Result<CommandResult> {
    let mut args = vec!["--build-upon-default-config"];

    if let Some(config) = config_path {
        args.push("--config");
        args.push(config);
    }

    run_command("detekt", &args)
}

/// Run detekt with auto-correct
pub fn detekt_fix(config_path: Option<&str>) -> Result<CommandResult> {
    let mut args = vec!["--auto-correct", "--build-upon-default-config"];

    if let Some(config) = config_path {
        args.push("--config");
        args.push(config);
    }

    run_command("detekt", &args)
}

/// Get ktlint version
pub fn ktlint_version() -> Result<String> {
    let result = run_command("ktlint", &["--version"])?;
    Ok(result.stdout.trim().to_string())
}

/// Get detekt version
pub fn detekt_version() -> Result<String> {
    let result = run_command("detekt", &["--version"])?;
    Ok(result.stdout.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_ktlint() {
        // Just verify it doesn't panic
        let _ = has_ktlint();
    }

    #[test]
    fn test_has_detekt() {
        let _ = has_detekt();
    }
}
