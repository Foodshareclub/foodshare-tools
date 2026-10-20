//! The cross-platform build engine.
//!
//! Builds the Skip app for iOS and Android: the Swift core, the Xcode
//! build, and the Gradle build. The Android build needs the Swift native
//! module map in place first, which is what
//! [`sync_android_native_modules`] writes.
//!
//! The Xcode build is unsigned and does not run the Skip action: the
//! toolchain builds the app, the CI signs it.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;

use serde::Serialize;
use tokio::process::Command;

/// The scheme the app is built with.
pub const SCHEME: &str = "FoodShare App";

/// The Xcode project, relative to the app directory.
pub const PROJECT: &str = "Darwin/FoodShare.xcodeproj";

/// The configuration a build produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Configuration {
    /// A debug build.
    Debug,
    /// A release build.
    Release,
}

impl Configuration {
    /// The name Xcode knows the configuration by.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Debug => "Debug",
            Self::Release => "Release",
        }
    }
}

/// The variant an Android build produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// A debug build.
    Debug,
    /// A release build.
    Release,
}

impl Variant {
    /// The Gradle task for the variant.
    #[must_use]
    pub fn task(self) -> &'static str {
        match self {
            Self::Debug => "assembleDebug",
            Self::Release => "assembleRelease",
        }
    }

    /// The variant's name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }
}

/// Where the app's build artefacts live.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The app repository.
    pub app_dir: PathBuf,
    /// The DerivedData directory.
    pub derived_data: PathBuf,
    /// The Android project directory.
    pub android_dir: PathBuf,
}

impl Layout {
    /// The layout for an app repository.
    #[must_use]
    pub fn new(app_dir: impl Into<PathBuf>) -> Self {
        let app_dir = app_dir.into();
        Self {
            derived_data: app_dir.join(".build").join("DerivedData"),
            android_dir: app_dir.join("Android"),
            app_dir,
        }
    }

    /// The Xcode project.
    #[must_use]
    pub fn project(&self) -> PathBuf {
        self.app_dir.join(PROJECT)
    }

    /// The built `.app` bundle.
    #[must_use]
    pub fn app_bundle(&self, configuration: Configuration) -> PathBuf {
        self.derived_data
            .join("Build")
            .join("Products")
            .join(format!("{}-iphonesimulator", configuration.name()))
            .join("FoodShare.app")
    }
}

/// How a build ended.
#[derive(Debug, Clone, Serialize)]
pub struct BuildOutcome {
    /// What was built.
    pub target: String,
    /// Whether it succeeded.
    pub success: bool,
    /// How long it took.
    pub duration_ms: u64,
    /// Where the artefact was written.
    pub artifact: Option<PathBuf>,
    /// Why it failed.
    pub error: Option<String>,
}

/// Build the Swift core.
pub async fn build_swift_core(layout: &Layout) -> BuildOutcome {
    run("swift", &["build"], &layout.app_dir, "Swift core", None).await
}

/// Run the Swift core unit tests.
pub async fn run_swift_tests(layout: &Layout) -> BuildOutcome {
    run("swift", &["test"], &layout.app_dir, "Swift tests", None).await
}

/// Build the iOS app.
///
/// The destination is either a device identifier, for a build that is
/// about to be installed, or a generic simulator destination.
pub async fn build_ios(
    layout: &Layout,
    configuration: Configuration,
    clean: bool,
    destination: &str,
) -> BuildOutcome {
    let mut args = vec![
        "-scheme".to_string(),
        SCHEME.to_string(),
        "-project".to_string(),
        layout.project().display().to_string(),
        "-destination".to_string(),
        destination.to_string(),
        "-configuration".to_string(),
        configuration.name().to_string(),
        "-derivedDataPath".to_string(),
        layout.derived_data.display().to_string(),
        "SKIP_ACTION=none".to_string(),
        "CODE_SIGN_IDENTITY=".to_string(),
        "CODE_SIGNING_REQUIRED=NO".to_string(),
        "CODE_SIGNING_ALLOWED=NO".to_string(),
    ];
    if clean {
        args.push("clean".to_string());
    }
    args.push("build".to_string());

    let outcome = run(
        "xcodebuild",
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
        &layout.app_dir,
        "iOS build",
        None,
    )
    .await;
    let artifact = layout.app_bundle(configuration);
    let success = outcome.success && artifact.is_dir();
    BuildOutcome {
        target: "ios".to_string(),
        success,
        duration_ms: outcome.duration_ms,
        artifact: success.then_some(artifact),
        error: if success {
            None
        } else {
            outcome
                .error
                .or_else(|| Some("the app bundle was not produced".to_string()))
        },
    }
}

/// Build the Android app.
pub async fn build_android(layout: &Layout, variant: Variant) -> BuildOutcome {
    if !layout.android_dir.is_dir() {
        return BuildOutcome {
            target: "android".to_string(),
            success: true,
            duration_ms: 0,
            artifact: None,
            error: None,
        };
    }

    sync_android_native_modules(layout);

    let mut env = std::env::vars().collect::<Vec<_>>();
    // The Skip CLI lives outside the default PATH on a Homebrew install.
    let skip = Path::new("/opt/homebrew/bin/skip");
    if skip.exists() {
        env.push((
            "SKIP_COMMAND_OVERRIDE".to_string(),
            skip.display().to_string(),
        ));
    }
    if let Some(sdk) = resolve_android_sdk() {
        env.push(("ANDROID_HOME".to_string(), sdk.display().to_string()));
        env.push(("ANDROID_SDK_ROOT".to_string(), sdk.display().to_string()));
    }
    let products = layout
        .derived_data
        .join("Build")
        .join("Products")
        .join("Debug-iphonesimulator");
    if products.is_dir() {
        env.push((
            "BUILT_PRODUCTS_DIR".to_string(),
            products.display().to_string(),
        ));
    }

    let outcome = run(
        &layout.android_dir.join("gradlew").display().to_string(),
        &[variant.task()],
        &layout.android_dir,
        "Android build",
        Some(env),
    )
    .await;

    let apk = layout
        .app_dir
        .join(".build")
        .join("Android")
        .join("app")
        .join("outputs")
        .join("apk")
        .join(variant.name())
        .join(format!("app-{}.apk", variant.name()));

    BuildOutcome {
        target: "android".to_string(),
        success: outcome.success,
        duration_ms: outcome.duration_ms,
        artifact: apk.is_file().then_some(apk),
        error: outcome.error,
    }
}

/// Run the Kotlin unit tests.
pub async fn run_kotlin_tests(layout: &Layout) -> BuildOutcome {
    if !layout.android_dir.is_dir() {
        return BuildOutcome {
            target: "kotlin".to_string(),
            success: true,
            duration_ms: 0,
            artifact: None,
            error: None,
        };
    }
    run(
        &layout.android_dir.join("gradlew").display().to_string(),
        &["test", "--quiet"],
        &layout.android_dir,
        "Kotlin tests",
        None,
    )
    .await
}

/// The Skip bridges, and the tests that cover them.
pub const BRIDGES: &[(&str, &str)] = &[
    ("ValidationBridge", "com.foodshare.core.validation.*"),
    ("GeoIntelligenceBridge", "com.foodshare.core.geo.*"),
    ("ContentModerationBridge", "com.foodshare.core.moderation.*"),
];

/// Run the Skip cross-platform bridge tests.
///
/// A bridge with no tests is not a failure: the report says so, and the
/// run continues.
pub async fn run_bridge_tests(layout: &Layout) -> Vec<BuildOutcome> {
    if !layout.android_dir.is_dir() {
        return Vec::new();
    }
    let gradlew = layout.android_dir.join("gradlew");
    let mut outcomes = Vec::new();
    for (name, target) in BRIDGES {
        let outcome = run(
            &gradlew.display().to_string(),
            &[":app:testDebugUnitTest", "--tests", target, "--quiet"],
            &layout.android_dir,
            name,
            None,
        )
        .await;
        outcomes.push(BuildOutcome {
            target: (*name).to_string(),
            success: outcome.success,
            duration_ms: outcome.duration_ms,
            artifact: None,
            error: outcome.error,
        });
    }
    outcomes
}

/// The Android SDK, if one is installed.
#[must_use]
pub fn resolve_android_sdk() -> Option<PathBuf> {
    for name in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(value) = std::env::var_os(name) {
            let path = PathBuf::from(value);
            if path.is_dir() {
                return Some(path);
            }
        }
    }
    let default = std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join("Library")
            .join("Android")
            .join("sdk")
    })?;
    default.is_dir().then_some(default)
}

/// Write the Swift native module map the Android build needs.
///
/// The Swift side of the app is compiled into native libraries, and the
/// Android side needs a module map to find them. Without it the Gradle
/// build cannot resolve the JNI and NDK headers.
pub fn sync_android_native_modules(layout: &Layout) {
    let module_map = "\
module CJNI [system] {
    header \"jni.h\"
    link \"android\"
    export *
}

module AndroidNDK [system] {
    header \"ndk.h\"
    link \"android\"
    export *
}

module ConcurrencyRuntimeC [system] {
    header \"include.h\"
    export *
}
";

    let checkouts = layout.derived_data.join("SourcePackages").join("checkouts");
    let jni = checkouts.join("swift-jni/Sources/CJNI/include/jni.h");
    let ndk = checkouts.join("swift-android-native/Sources/AndroidNDK/include/ndk.h");
    let include =
        checkouts.join("swift-android-native/Sources/ConcurrencyRuntimeC/include/include.h");

    let targets = [
        layout
            .app_dir
            .join(".build/Android/FoodShare/swift/aarch64-unknown-linux-android28/debug/Modules"),
        layout.derived_data.join(
            "Build/Intermediates.noindex/BuildToolPluginIntermediates/foodshare-app.output/FoodShare/skipstone/FoodShare/build/swift/aarch64-unknown-linux-android28/debug/Modules",
        ),
    ];

    for target in targets {
        if std::fs::create_dir_all(&target).is_err() {
            continue;
        }
        if std::fs::write(target.join("module.modulemap"), module_map).is_err() {
            continue;
        }
        for (source, name) in [(&jni, "jni.h"), (&ndk, "ndk.h"), (&include, "include.h")] {
            if source.exists() {
                let _ = std::fs::copy(source, target.join(name));
            }
        }
    }

    // A stale single-producer intermediate confuses the incremental
    // build, so it goes.
    let stale = layout
        .app_dir
        .join(".build/Android/FoodShare/swift/aarch64-unknown-linux-android28/debug/SkipAndroidBridge.build");
    if stale.exists() {
        let _ = std::fs::remove_dir_all(stale);
    }
}

/// Remove the build artefacts.
#[must_use]
pub fn clean_artifacts(layout: &Layout) -> Vec<PathBuf> {
    let paths = [
        layout.app_dir.join(".build"),
        layout.app_dir.join("Darwin").join("build"),
        layout.android_dir.join("app").join("build"),
        layout.android_dir.join(".gradle"),
    ];
    let mut removed = Vec::new();
    for path in paths {
        if path.exists() && std::fs::remove_dir_all(&path).is_ok() {
            removed.push(path);
        }
    }
    removed
}

/// Run a command, streaming its output to the terminal.
async fn run(
    program: &str,
    args: &[&str],
    cwd: &Path,
    target: &str,
    env: Option<Vec<(String, String)>>,
) -> BuildOutcome {
    let started = Instant::now();
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(env) = env {
        command.env_clear();
        command.envs(env);
    }

    match command.status().await {
        Ok(status) if status.success() => BuildOutcome {
            target: target.to_string(),
            success: true,
            duration_ms: started.elapsed().as_millis() as u64,
            artifact: None,
            error: None,
        },
        Ok(status) => BuildOutcome {
            target: target.to_string(),
            success: false,
            duration_ms: started.elapsed().as_millis() as u64,
            artifact: None,
            error: Some(format!("exited with {}", status.code().unwrap_or(-1))),
        },
        Err(e) => BuildOutcome {
            target: target.to_string(),
            success: false,
            duration_ms: started.elapsed().as_millis() as u64,
            artifact: None,
            error: Some(format!("could not run {program}: {e}")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout::new("/tmp/app")
    }

    #[test]
    fn test_layout_paths() {
        let layout = layout();
        assert!(layout.project().ends_with("Darwin/FoodShare.xcodeproj"));
        assert!(
            layout
                .app_bundle(Configuration::Debug)
                .ends_with("Debug-iphonesimulator/FoodShare.app")
        );
        assert!(layout.android_dir.ends_with("Android"));
    }

    #[test]
    fn test_configuration_names() {
        assert_eq!(Configuration::Debug.name(), "Debug");
        assert_eq!(Configuration::Release.name(), "Release");
    }

    #[test]
    fn test_variant_tasks() {
        assert_eq!(Variant::Debug.task(), "assembleDebug");
        assert_eq!(Variant::Release.task(), "assembleRelease");
    }

    #[test]
    fn test_clean_artifacts_of_a_missing_tree_is_empty() {
        let dir = tempfile::tempdir().expect("temp dir");
        let layout = Layout::new(dir.path());
        assert!(clean_artifacts(&layout).is_empty());
    }

    #[tokio::test]
    async fn test_build_android_without_the_project_succeeds() {
        let dir = tempfile::tempdir().expect("temp dir");
        let layout = Layout::new(dir.path());
        let outcome = build_android(&layout, Variant::Debug).await;
        assert!(outcome.success);
    }
}
