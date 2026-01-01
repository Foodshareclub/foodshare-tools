//! Gradle build system integration
//!
//! Provides wrappers for Gradle commands.

use foodshare_core::error::Result;
use foodshare_core::process::{run_command_in_dir, CommandResult};
use std::path::Path;

/// Run a Gradle task
pub fn run_task(project_dir: &Path, task: &str) -> Result<CommandResult> {
    let gradle_wrapper = if cfg!(windows) {
        "gradlew.bat"
    } else {
        "./gradlew"
    };

    run_command_in_dir(gradle_wrapper, &[task], project_dir)
}

/// Build debug APK
pub fn build_debug(project_dir: &Path) -> Result<CommandResult> {
    run_task(project_dir, "assembleDebug")
}

/// Build release APK
pub fn build_release(project_dir: &Path) -> Result<CommandResult> {
    run_task(project_dir, "assembleRelease")
}

/// Build debug bundle (AAB)
pub fn bundle_debug(project_dir: &Path) -> Result<CommandResult> {
    run_task(project_dir, "bundleDebug")
}

/// Build release bundle (AAB)
pub fn bundle_release(project_dir: &Path) -> Result<CommandResult> {
    run_task(project_dir, "bundleRelease")
}

/// Run unit tests
pub fn test(project_dir: &Path) -> Result<CommandResult> {
    run_task(project_dir, "test")
}

/// Run connected (instrumented) tests
pub fn connected_test(project_dir: &Path) -> Result<CommandResult> {
    run_task(project_dir, "connectedAndroidTest")
}

/// Clean build artifacts
pub fn clean(project_dir: &Path) -> Result<CommandResult> {
    run_task(project_dir, "clean")
}

/// Check for dependency updates
pub fn dependency_updates(project_dir: &Path) -> Result<CommandResult> {
    run_task(project_dir, "dependencyUpdates")
}

/// Run lint checks
pub fn lint(project_dir: &Path) -> Result<CommandResult> {
    run_task(project_dir, "lint")
}

/// Run detekt static analysis
pub fn detekt(project_dir: &Path) -> Result<CommandResult> {
    run_task(project_dir, "detekt")
}

/// Sync Gradle dependencies
pub fn sync(project_dir: &Path) -> Result<CommandResult> {
    run_task(project_dir, "--refresh-dependencies")
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_gradle_wrapper_path() {
        let wrapper = if cfg!(windows) {
            "gradlew.bat"
        } else {
            "./gradlew"
        };
        assert!(!wrapper.is_empty());
    }
}
