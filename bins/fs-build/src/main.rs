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
use clap::{Parser, Subcommand};

use foodshare_core::error::exit_codes;
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

    /// Build the Android app.
    #[arg(long)]
    android: bool,

    /// Build the iOS app.
    #[arg(long)]
    ios: bool,

    /// Build both platforms. This is the default.
    #[arg(long)]
    all: bool,

    /// Build the Swift package core.
    #[arg(long)]
    swift: bool,

    /// Build the Release configuration instead of Debug.
    #[arg(long)]
    release: bool,

    /// Clean the build artefacts before building.
    #[arg(long)]
    clean: bool,

    /// Run every test suite.
    #[arg(long)]
    test: bool,

    /// The test suite to run.
    #[command(subcommand)]
    suite: Option<Suite>,
}

/// The test suites the engine can run.
#[derive(Subcommand, Debug)]
enum Suite {
    /// The Swift unit tests.
    Swift,
    /// The Kotlin unit tests.
    Kotlin,
    /// The Skip cross-platform bridge tests.
    Bridge,
    /// Every test suite.
    Test,
}

/// What the engine was asked to do.
#[derive(Debug, PartialEq, Eq)]
enum Action {
    /// Build the Swift package core.
    SwiftCore,
    /// Run the Swift unit tests.
    SwiftTests,
    /// Run the Kotlin unit tests.
    KotlinTests,
    /// Run the Skip cross-platform bridge tests.
    BridgeTests,
    /// Run every test suite.
    AllTests,
    /// Build the app.
    Build,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let workspace = match cli.app_dir.as_deref() {
        Some(app) => Workspace::with_app_dir(app),
        None => Workspace::discover()?,
    };
    let layout = Layout::new(workspace.app());

    let action = match cli.suite {
        Some(Suite::Swift) => Action::SwiftTests,
        Some(Suite::Kotlin) => Action::KotlinTests,
        Some(Suite::Bridge) => Action::BridgeTests,
        Some(Suite::Test) => Action::AllTests,
        None if cli.test => Action::AllTests,
        None if cli.swift => Action::SwiftCore,
        None => Action::Build,
    };

    let is_release = cli.release;
    let clean = cli.clean;
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
    match action {
        Action::SwiftTests => {
            return report("Swift tests", build::run_swift_tests(&layout).await);
        }
        Action::KotlinTests => {
            return report("Kotlin tests", build::run_kotlin_tests(&layout).await);
        }
        Action::BridgeTests => {
            let outcomes = build::run_bridge_tests(&layout).await;
            let mut all_ok = true;
            for outcome in &outcomes {
                let icon = if outcome.success { "✓" } else { "⚠" };
                println!("  {icon} {}", outcome.target);
                all_ok &= outcome.success;
            }
            if !all_ok {
                std::process::exit(exit_codes::VALIDATION_ERROR);
            }
            return Ok(());
        }
        Action::AllTests => {
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
                std::process::exit(exit_codes::VALIDATION_ERROR);
            }
            return Ok(());
        }
        Action::SwiftCore => {
            return report("Swift core", build::build_swift_core(&layout).await);
        }
        Action::Build => {}
    }

    // The builds.
    let android_only = cli.android && !cli.ios && !cli.all;
    let ios_only = cli.ios && !cli.android && !cli.all;
    let both = cli.all || (!android_only && !ios_only);

    if ios_only || both {
        let outcome = build::build_ios(
            &layout,
            configuration,
            clean,
            "generic/platform=iOS Simulator",
        )
        .await;
        report("iOS build", outcome)?;
    }

    if android_only || both {
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
