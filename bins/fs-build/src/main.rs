//! fs-build — the cross-platform build engine.
//!
//! Builds the Skip app for iOS and Android, and runs the Swift, Kotlin
//! and bridge test suites.
//!
//! Usage:
//!   fs-build                  # both platforms
//!   fs-build --ios            # the iOS app
//!   fs-build --android        # the Android app
//!   fs-build --all            # both platforms
//!   fs-build --swift          # the Swift core
//!   fs-build --release        # a release build
//!   fs-build --clean          # clean the artefacts first
//!   fs-build swift            # the Swift unit tests
//!   fs-build kotlin           # the Kotlin unit tests
//!   fs-build bridge           # the Skip bridge tests
//!   fs-build test             # every test suite

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use foodshare_core::workspace::Workspace;
use fs_app::build::{self, Configuration, Layout, Variant};

#[derive(Parser, Debug)]
#[command(
    name = "fs-build",
    version,
    about = "FoodShare cross-platform build engine"
)]
struct Cli {
    /// The app repository. Defaults to the directory that holds
    /// `foodshare-app`.
    #[arg(long, global = true)]
    app_dir: Option<PathBuf>,

    /// The arguments, in the order the Bun builder took them.
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let args = &cli.args;
    let workspace = match cli.app_dir.as_deref() {
        Some(app) => Workspace::with_app_dir(app),
        None => Workspace::discover()?,
    };
    let layout = Layout::new(workspace.app());

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    let has = |flag: &str| args.iter().any(|a| a == flag);
    let is_release = has("--release");
    let clean = has("--clean");
    let configuration = if is_release {
        Configuration::Release
    } else {
        Configuration::Debug
    };
    let variant = if is_release {
        Variant::Release
    } else {
        Variant::Debug
    };

    println!("🍎 =========================================================");
    println!("🍎 FoodShare Universal Cross-Platform Builder & Tester");
    println!("🍎 =========================================================");

    // The disk is checked before a large compilation, not after a
    // failure.
    if let Ok(stats) = foodshare_core::disk::workspace_stats(&layout.app_dir) {
        println!("💾 {} GB free ({}%)", stats.avail_gb, stats.avail_percent);
    }

    if clean {
        let removed = build::clean_artifacts(&layout);
        println!("🧹 cleaned {} build director(ies)", removed.len());
    }

    // The test suites.
    if has("swift") && !has("--swift") {
        return report("Swift tests", build::run_swift_tests(&layout).await);
    }
    if has("kotlin") && !has("--kotlin") {
        return report("Kotlin tests", build::run_kotlin_tests(&layout).await);
    }
    if has("bridge") && !has("--bridge") {
        let outcomes = build::run_bridge_tests(&layout).await;
        for outcome in &outcomes {
            let icon = if outcome.success { "✓" } else { "⚠" };
            println!("  {icon} {}", outcome.target);
        }
        return Ok(());
    }
    if has("test") || has("--test") || (has("all") && !has("--all")) {
        println!("🧪 Running the full cross-platform test suite...");
        let swift = build::run_swift_tests(&layout).await;
        let kotlin = build::run_kotlin_tests(&layout).await;
        let bridges = build::run_bridge_tests(&layout).await;
        let all_ok = swift.success && kotlin.success;
        println!(
            "\n{} Test Summary: Swift ({}), Kotlin ({})",
            if all_ok { "✅" } else { "❌" },
            if swift.success { "PASSED" } else { "FAILED" },
            if kotlin.success { "PASSED" } else { "FAILED" }
        );
        for outcome in &bridges {
            let icon = if outcome.success { "✓" } else { "⚠" };
            println!("  {icon} {}", outcome.target);
        }
        if !all_ok {
            std::process::exit(1);
        }
        return Ok(());
    }

    // The builds.
    let android_only =
        (has("--android") || has("android")) && !has("--ios") && !has("ios") && !has("--all");
    let ios_only =
        (has("--ios") || has("ios")) && !has("--android") && !has("android") && !has("--all");
    let explicit_swift = has("--swift") || has("swift");

    let build_ios = ios_only || (!android_only && !explicit_swift);
    let build_android = android_only || (!ios_only && !explicit_swift);

    if explicit_swift {
        return report("Swift core", build::build_swift_core(&layout).await);
    }

    if build_ios {
        let outcome = build::build_ios(
            &layout,
            configuration,
            clean,
            "generic/platform=iOS Simulator",
        )
        .await;
        report("iOS build", outcome)?;
    }

    if build_android {
        report(
            "Android build",
            build::build_android(&layout, variant).await,
        )?;
    }

    println!("\n==========================================");
    println!("🎉 FoodShare Cross-Platform Build Complete!");
    println!("==========================================\n");
    Ok(())
}

/// Print the outcome of one build, and exit when it failed.
fn report(label: &str, outcome: build::BuildOutcome) -> Result<()> {
    if outcome.success {
        let took = outcome.duration_ms as f64 / 1000.0;
        match &outcome.artifact {
            Some(artifact) => println!("✓ {label} complete ({took:.1}s) -> {}", artifact.display()),
            None => println!("✓ {label} complete ({took:.1}s)"),
        }
        Ok(())
    } else {
        anyhow::bail!(
            "❌ {label} failed: {}",
            outcome.error.unwrap_or_else(|| "unknown error".to_string())
        )
    }
}

fn print_help() {
    println!(
        "
Usage: fs-build [options]

Build Modes:
  --android              Build Android debug APK (assembleDebug)
  --ios                  Build iOS simulator app
  --all                  Build both iOS and Android
  --swift                Build Swift SPM core
  --release              Build Release configuration instead of Debug
  --clean                Clean build artifacts before building

Test Modes:
  --test, test, all      Run full cross-platform test suite (Swift + Kotlin + Bridges)
  swift                  Run Swift unit tests
  kotlin                 Run Kotlin unit tests
  bridge                 Run Skip cross-platform bridge tests
"
    );
}
