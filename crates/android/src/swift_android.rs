//! Swift cross-compilation for Android
//!
//! Provides tools for building Swift code for Android using the Swift SDK.

use foodshare_core::error::Result;
use foodshare_core::process::{command_exists, run_command, run_command_in_dir, CommandResult};
use std::path::Path;

/// Target architecture for Android
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AndroidTarget {
    /// ARM64 for physical devices
    Arm64,
    /// x86_64 for emulator
    X86_64,
}

impl AndroidTarget {
    /// Get the Swift triple for this target
    pub fn swift_triple(&self) -> &'static str {
        match self {
            AndroidTarget::Arm64 => "aarch64-unknown-linux-android",
            AndroidTarget::X86_64 => "x86_64-unknown-linux-android",
        }
    }

    /// Get the NDK architecture name
    pub fn ndk_arch(&self) -> &'static str {
        match self {
            AndroidTarget::Arm64 => "arm64-v8a",
            AndroidTarget::X86_64 => "x86_64",
        }
    }
}

/// Check if Swift is available
pub fn has_swift() -> bool {
    command_exists("swift")
}

/// Check if swift-java is available
pub fn has_swift_java() -> bool {
    command_exists("swift-java")
}

/// Build Swift package for Android
pub fn build(
    package_dir: &Path,
    target: AndroidTarget,
    configuration: &str,
    api_level: u8,
) -> Result<CommandResult> {
    let triple = target.swift_triple();
    let sdk_id = format!("android{}-{}", api_level, target.ndk_arch());

    run_command_in_dir(
        "swift",
        &[
            "build",
            "-c",
            configuration,
            "--triple",
            triple,
            "--sdk",
            &sdk_id,
        ],
        package_dir,
    )
}

/// Generate Java/Kotlin bindings from Swift
pub fn generate_bindings(
    sources_dir: &Path,
    output_dir: &Path,
    package_name: &str,
) -> Result<CommandResult> {
    run_command(
        "swift-java",
        &[
            "generate",
            "--sources",
            &sources_dir.to_string_lossy(),
            "--output",
            &output_dir.to_string_lossy(),
            "--package",
            package_name,
        ],
    )
}

/// Run Swift tests on host (not cross-compiled)
pub fn test_host(package_dir: &Path, filter: Option<&str>) -> Result<CommandResult> {
    let mut args = vec!["test"];

    if let Some(f) = filter {
        args.push("--filter");
        args.push(f);
    }

    run_command_in_dir("swift", &args, package_dir)
}

/// Get Swift version
pub fn swift_version() -> Result<String> {
    let result = run_command("swift", &["--version"])?;
    Ok(result.stdout.lines().next().unwrap_or("Unknown").to_string())
}

/// Verify swift-java installation
pub fn verify_swift_java() -> Result<CommandResult> {
    run_command("swift-java", &["--version"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_android_target_triple() {
        assert_eq!(
            AndroidTarget::Arm64.swift_triple(),
            "aarch64-unknown-linux-android"
        );
        assert_eq!(
            AndroidTarget::X86_64.swift_triple(),
            "x86_64-unknown-linux-android"
        );
    }

    #[test]
    fn test_android_target_ndk_arch() {
        assert_eq!(AndroidTarget::Arm64.ndk_arch(), "arm64-v8a");
        assert_eq!(AndroidTarget::X86_64.ndk_arch(), "x86_64");
    }
}
