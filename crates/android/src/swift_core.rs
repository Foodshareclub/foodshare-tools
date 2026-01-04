//! FoodshareCore Swift build for Android
//!
//! Rust implementation of the build-android.sh script from foodshare-core.
//! Builds FoodshareCore Swift library for Android using Swift SDK.
//!
//! # Usage
//!
//! ```bash
//! # Build for all architectures (debug)
//! foodshare-android swift-core build --target all
//!
//! # Build for ARM64 only (release)
//! foodshare-android swift-core build --target arm64 --configuration release
//!
//! # Check prerequisites
//! foodshare-android swift-core check
//! ```

use foodshare_core::error::Result;
use foodshare_core::process::{command_exists, run_command, run_command_in_dir};
use owo_colors::OwoColorize;
use std::path::{Path, PathBuf};

/// Default Android API level for Swift SDK (matches Swift 6.2 SDK)
pub const DEFAULT_API_LEVEL: u8 = 24;

/// Target architecture for Swift cross-compilation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SwiftAndroidTarget {
    /// ARM64 for physical Android devices
    Arm64,
    /// x86_64 for Android emulator
    X86_64,
}

impl SwiftAndroidTarget {
    /// Get the Swift SDK identifier for Swift 6.2
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

    /// Get all supported targets
    pub fn all() -> &'static [SwiftAndroidTarget] {
        &[SwiftAndroidTarget::Arm64, SwiftAndroidTarget::X86_64]
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
    /// Path to Android project for auto-copy (optional)
    pub android_project_dir: Option<PathBuf>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            project_dir: PathBuf::from("."),
            output_dir: PathBuf::from("android-libs"),
            api_level: DEFAULT_API_LEVEL,
            configuration: "debug".to_string(),
            static_stdlib: false, // Match shell script behavior
            android_project_dir: None,
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

    let mut args = vec!["build", "--swift-sdk", &sdk_id];

    if config.static_stdlib {
        args.push("--static-swift-stdlib");
    }

    if config.configuration == "release" {
        args.push("-c");
        args.push("release");
    }

    // Use /usr/bin/swift to ensure we use the system Swift (matches shell script)
    let swift_cmd = if Path::new("/usr/bin/swift").exists() {
        "/usr/bin/swift"
    } else {
        "swift"
    };

    let result = run_command_in_dir(swift_cmd, &args, &config.project_dir)?;

    if !result.success {
        return Ok(BuildResult {
            target,
            success: false,
            output_path: None,
            error: Some(result.stderr),
        });
    }

    // Find the output library using glob pattern (matches shell script behavior)
    let output_path = find_and_copy_library(target, config, &sdk_id)?;

    match output_path {
        Some(path) => {
            println!("  {} Built: {}", "✓".green(), path.display());
            Ok(BuildResult {
                target,
                success: true,
                output_path: Some(path),
                error: None,
            })
        }
        None => {
            println!("  {} Library not found for {}", "⚠".yellow(), target.jni_arch());
            Ok(BuildResult {
                target,
                success: false,
                output_path: None,
                error: Some(format!("Library not found for {}", target.jni_arch())),
            })
        }
    }
}

/// Find and copy the built library to output directory
fn find_and_copy_library(
    target: SwiftAndroidTarget,
    config: &BuildConfig,
    sdk_id: &str,
) -> Result<Option<PathBuf>> {
    let lib_name = "libFoodshareCore.so";
    let build_dir = config.project_dir.join(".build");

    // Search for the library in the build directory (matches shell script: find .build -name "libFoodshareCore.so" -path "*$sdk*")
    let lib_path = find_library_in_build(&build_dir, lib_name, sdk_id)?;

    if let Some(source_path) = lib_path {
        let output_arch_dir = config.output_dir.join(target.jni_arch());
        std::fs::create_dir_all(&output_arch_dir)?;

        let output_path = output_arch_dir.join(lib_name);
        std::fs::copy(&source_path, &output_path)?;

        return Ok(Some(output_path));
    }

    Ok(None)
}

/// Recursively find library in build directory matching SDK pattern
fn find_library_in_build(
    build_dir: &Path,
    lib_name: &str,
    sdk_pattern: &str,
) -> Result<Option<PathBuf>> {
    if !build_dir.exists() {
        return Ok(None);
    }

    for entry in walkdir::WalkDir::new(build_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name() {
                if name == lib_name {
                    // Check if path contains the SDK pattern
                    if path.to_string_lossy().contains(sdk_pattern) {
                        return Ok(Some(path.to_path_buf()));
                    }
                }
            }
        }
    }

    Ok(None)
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
    let mut results = Vec::new();

    // Clean output directory (matches shell script: rm -rf "$OUTPUT_DIR")
    if config.output_dir.exists() {
        std::fs::remove_dir_all(&config.output_dir)?;
    }
    std::fs::create_dir_all(&config.output_dir)?;

    for target in SwiftAndroidTarget::all() {
        let result = build_for_target(*target, config)?;
        results.push(result);
    }

    // Auto-copy to Android project if configured (matches shell script behavior)
    if let Some(ref android_dir) = config.android_project_dir {
        if android_dir.exists() {
            println!();
            println!("  {} Copying to Android project...", "->".blue());
            copy_to_android_project(&config.output_dir, android_dir)?;
        }
    }

    Ok(results)
}

/// Build for a single target by name
pub fn build_single(target_name: &str, config: &BuildConfig) -> Result<BuildResult> {
    let target = match target_name.to_lowercase().as_str() {
        "arm64" | "arm64-v8a" | "aarch64" => SwiftAndroidTarget::Arm64,
        "x86_64" | "x86-64" => SwiftAndroidTarget::X86_64,
        _ => {
            return Err(foodshare_core::error::Error::new(
                foodshare_core::error::ErrorCode::InvalidInput,
                format!("Unknown target: {}. Use arm64 or x86_64", target_name),
            ));
        }
    };

    // Clean output directory
    if config.output_dir.exists() {
        std::fs::remove_dir_all(&config.output_dir)?;
    }
    std::fs::create_dir_all(&config.output_dir)?;

    let result = build_for_target(target, config)?;

    // Auto-copy to Android project if configured
    if result.success {
        if let Some(ref android_dir) = config.android_project_dir {
            if android_dir.exists() {
                println!();
                println!("  {} Copying to Android project...", "->".blue());
                copy_to_android_project(&config.output_dir, android_dir)?;
            }
        }
    }

    Ok(result)
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
                        "✓".green(),
                        dest_path.display()
                    );
                }
            }
        }
    }

    Ok(())
}

/// Detect Android project relative to FoodshareCore
pub fn detect_android_project(project_dir: &Path) -> Option<PathBuf> {
    // Check ../foodshare-android (matches shell script)
    let android_dir = project_dir.join("../foodshare-android");
    if android_dir.exists() {
        return Some(android_dir);
    }
    None
}

/// Print setup instructions
pub fn print_setup_instructions() {
    println!();
    println!("{}", "Swift SDK for Android Setup".bold());
    println!();
    println!("1. Install Swift 6.2+ toolchain:");
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
    println!("   Or from foodshare-core directory:");
    println!("   foodshare-android swift-core build --target all --project-dir .");
    println!();
}

/// Main entry point matching shell script behavior
///
/// Equivalent to: ./scripts/build-android.sh [arm64|x86_64|all] [debug|release]
pub fn build_android(
    arch: &str,
    configuration: &str,
    project_dir: &Path,
) -> Result<Vec<BuildResult>> {
    println!("{}", "Building FoodshareCore for Android...".bold());
    println!("Architecture: {}", arch);
    println!("Configuration: {}", configuration);
    println!();

    let output_dir = project_dir.join("android-libs");
    let android_project_dir = detect_android_project(project_dir);

    let config = BuildConfig {
        project_dir: project_dir.to_path_buf(),
        output_dir,
        api_level: DEFAULT_API_LEVEL,
        configuration: configuration.to_string(),
        static_stdlib: false,
        android_project_dir,
    };

    let results = match arch.to_lowercase().as_str() {
        "all" => build_all(&config)?,
        target => vec![build_single(target, &config)?],
    };

    println!();
    println!("{}", "Build complete!".green().bold());

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swift_android_target_sdk_id() {
        assert_eq!(
            SwiftAndroidTarget::Arm64.sdk_id(24),
            "aarch64-unknown-linux-android24"
        );
        assert_eq!(
            SwiftAndroidTarget::X86_64.sdk_id(24),
            "x86_64-unknown-linux-android24"
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
        assert_eq!(config.api_level, DEFAULT_API_LEVEL);
        assert_eq!(config.configuration, "debug");
        assert!(!config.static_stdlib); // Matches shell script
        assert!(config.android_project_dir.is_none());
    }

    #[test]
    fn test_all_targets() {
        let targets = SwiftAndroidTarget::all();
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&SwiftAndroidTarget::Arm64));
        assert!(targets.contains(&SwiftAndroidTarget::X86_64));
    }

    #[test]
    fn test_target_display_names() {
        assert_eq!(SwiftAndroidTarget::Arm64.display_name(), "ARM64");
        assert_eq!(SwiftAndroidTarget::X86_64.display_name(), "x86_64");
    }
}
