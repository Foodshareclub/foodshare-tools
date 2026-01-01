//! FoodshareCore Swift build for Android
//!
//! Rust implementation of the build-android.sh and setup-android-sdk.sh scripts.

use foodshare_core::error::Result;
use foodshare_core::process::{command_exists, run_command, run_command_in_dir};
use owo_colors::OwoColorize;
use std::path::{Path, PathBuf};

/// Target architecture for Swift cross-compilation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SwiftAndroidTarget {
    /// ARM64 for physical Android devices
    Arm64,
    /// x86_64 for Android emulator
    X86_64,
}

impl SwiftAndroidTarget {
    /// Get the Swift SDK identifier
    pub fn sdk_id(&self, api_level: u8) -> String {
        match self {
            Self::Arm64 => format!("aarch64-unknown-linux-android{}", api_level),
            Self::X86_64 => format!("x86_64-unknown-linux-android{}", api_level),
        }
    }

    /// Get the JNI library architecture directory name
    pub fn jni_arch(&self) -> &'static str {
        match self {
            Self::Arm64 => "arm64-v8a",
            Self::X86_64 => "x86_64",
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Arm64 => "ARM64",
            Self::X86_64 => "x86_64",
        }
    }
}

/// Build configuration
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub project_dir: PathBuf,
    pub output_dir: PathBuf,
    pub api_level: u8,
    pub configuration: String,
    pub static_stdlib: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            project_dir: PathBuf::from("."),
            output_dir: PathBuf::from("android-libs"),
            api_level: 28,
            configuration: "debug".to_string(),
            static_stdlib: true,
        }
    }
}

/// Check prerequisites for building Swift for Android
pub fn check_prerequisites() -> Result<PrerequisiteStatus> {
    let mut status = PrerequisiteStatus::default();

    // Check Swift
    if command_exists("swift") {
        let result = run_command("swift", &["--version"])?;
        status.swift_version = Some(result.stdout.lines().next().unwrap_or("").to_string());
        status.swift_installed = true;
    }

    // Check Swift SDK for Android
    let sdk_result = run_command("swift", &["sdk", "list"]);
    if let Ok(result) = sdk_result {
        status.android_sdk_installed = result.stdout.contains("android");
    }

    // Check Android NDK
    status.ndk_path = std::env::var("ANDROID_NDK_HOME").ok().map(PathBuf::from);
    if let Some(ref path) = status.ndk_path {
        status.ndk_installed = path.exists();
    }

    Ok(status)
}

/// Prerequisite check status
#[derive(Debug, Default)]
pub struct PrerequisiteStatus {
    pub swift_installed: bool,
    pub swift_version: Option<String>,
    pub android_sdk_installed: bool,
    pub ndk_installed: bool,
    pub ndk_path: Option<PathBuf>,
}

impl PrerequisiteStatus {
    pub fn is_ready(&self) -> bool {
        self.swift_installed && self.android_sdk_installed && self.ndk_installed
    }

    pub fn print_status(&self) {
        println!("{}", "Prerequisites Check".bold());
        println!();

        if self.swift_installed {
            println!(
                "  {} Swift: {}",
                "OK".green(),
                self.swift_version.as_deref().unwrap_or("installed")
            );
        } else {
            println!("  {} Swift: not found", "ERROR".red());
        }

        if self.android_sdk_installed {
            println!("  {} Swift SDK for Android: installed", "OK".green());
        } else {
            println!("  {} Swift SDK for Android: not installed", "ERROR".red());
        }

        if self.ndk_installed {
            println!(
                "  {} Android NDK: {}",
                "OK".green(),
                self.ndk_path.as_ref().map(|p| p.display().to_string()).unwrap_or_default()
            );
        } else {
            println!("  {} Android NDK: not found (set ANDROID_NDK_HOME)", "ERROR".red());
        }
    }
}

/// Build FoodshareCore for a specific Android target
pub fn build_for_target(
    target: SwiftAndroidTarget,
    config: &BuildConfig,
) -> Result<BuildResult> {
    let sdk_id = target.sdk_id(config.api_level);

    println!(
        "  {} Building for {} ({})...",
        "->".blue(),
        target.display_name(),
        sdk_id
    );

    let mut args = vec![
        "build",
        "--swift-sdk",
        &sdk_id,
    ];

    if config.static_stdlib {
        args.push("--static-swift-stdlib");
    }

    if config.configuration == "release" {
        args.push("-c");
        args.push("release");
    }

    let result = run_command_in_dir("swift", &args, &config.project_dir)?;

    if !result.success {
        return Ok(BuildResult {
            target,
            success: false,
            output_path: None,
            error: Some(result.stderr),
        });
    }

    // Find and copy the output library
    let build_config_dir = &config.configuration;
    let lib_name = "libFoodshareCore.so";
    let build_path = config.project_dir
        .join(".build")
        .join(&sdk_id)
        .join(build_config_dir)
        .join(lib_name);

    let output_arch_dir = config.output_dir.join(target.jni_arch());
    std::fs::create_dir_all(&output_arch_dir)?;

    let output_path = output_arch_dir.join(lib_name);

    if build_path.exists() {
        std::fs::copy(&build_path, &output_path)?;
        println!(
            "  {} Built: {}",
            "OK".green(),
            output_path.display()
        );
        Ok(BuildResult {
            target,
            success: true,
            output_path: Some(output_path),
            error: None,
        })
    } else {
        // Try .dylib extension
        let dylib_path = build_path.with_extension("dylib");
        if dylib_path.exists() {
            std::fs::copy(&dylib_path, &output_path)?;
            println!(
                "  {} Built: {}",
                "OK".green(),
                output_path.display()
            );
            Ok(BuildResult {
                target,
                success: true,
                output_path: Some(output_path),
                error: None,
            })
        } else {
            Ok(BuildResult {
                target,
                success: false,
                output_path: None,
                error: Some(format!("Library not found at: {}", build_path.display())),
            })
        }
    }
}

/// Build result
#[derive(Debug)]
pub struct BuildResult {
    pub target: SwiftAndroidTarget,
    pub success: bool,
    pub output_path: Option<PathBuf>,
    pub error: Option<String>,
}

/// Build for all targets
pub fn build_all(config: &BuildConfig) -> Result<Vec<BuildResult>> {
    let targets = [SwiftAndroidTarget::Arm64, SwiftAndroidTarget::X86_64];
    let mut results = Vec::new();

    // Clean output directory
    if config.output_dir.exists() {
        std::fs::remove_dir_all(&config.output_dir)?;
    }
    std::fs::create_dir_all(&config.output_dir)?;

    for target in targets {
        let result = build_for_target(target, config)?;
        results.push(result);
    }

    Ok(results)
}

/// Copy built libraries to Android project
pub fn copy_to_android_project(
    output_dir: &Path,
    android_project_dir: &Path,
) -> Result<()> {
    let jni_libs_dir = android_project_dir.join("app/src/main/jniLibs");
    std::fs::create_dir_all(&jni_libs_dir)?;

    for entry in std::fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let arch_name = path.file_name().unwrap();
            let dest_dir = jni_libs_dir.join(arch_name);
            std::fs::create_dir_all(&dest_dir)?;

            for lib_entry in std::fs::read_dir(&path)? {
                let lib_entry = lib_entry?;
                let lib_path = lib_entry.path();
                if lib_path.extension().map_or(false, |e| e == "so") {
                    let dest_path = dest_dir.join(lib_path.file_name().unwrap());
                    std::fs::copy(&lib_path, &dest_path)?;
                    println!(
                        "  {} Copied to: {}",
                        "OK".green(),
                        dest_path.display()
                    );
                }
            }
        }
    }

    Ok(())
}

/// Print setup instructions
pub fn print_setup_instructions() {
    println!();
    println!("{}", "Swift SDK for Android Setup".bold());
    println!();
    println!("1. Install snapshot toolchain:");
    println!("   swiftly install main-snapshot-2025-12-17");
    println!("   swiftly use main-snapshot-2025-12-17");
    println!();
    println!("2. Install Swift SDK for Android:");
    println!("   Visit https://www.swift.org/download/ for the SDK URL");
    println!("   swift sdk install <URL> --checksum <SHA256>");
    println!();
    println!("3. Set Android NDK path:");
    println!("   export ANDROID_NDK_HOME=/path/to/android-ndk-r27d");
    println!();
    println!("4. Build FoodshareCore:");
    println!("   foodshare-android swift-core build --target all");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swift_android_target_sdk_id() {
        assert_eq!(
            SwiftAndroidTarget::Arm64.sdk_id(28),
            "aarch64-unknown-linux-android28"
        );
        assert_eq!(
            SwiftAndroidTarget::X86_64.sdk_id(28),
            "x86_64-unknown-linux-android28"
        );
    }

    #[test]
    fn test_swift_android_target_jni_arch() {
        assert_eq!(SwiftAndroidTarget::Arm64.jni_arch(), "arm64-v8a");
        assert_eq!(SwiftAndroidTarget::X86_64.jni_arch(), "x86_64");
    }

    #[test]
    fn test_build_config_default() {
        let config = BuildConfig::default();
        assert_eq!(config.api_level, 28);
        assert_eq!(config.configuration, "debug");
        assert!(config.static_stdlib);
    }
}
