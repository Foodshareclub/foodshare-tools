//! fs-sim — the simulator runner.
//!
//! Boots a simulator, builds the app for it, installs it, launches it,
//! takes a screenshot, and runs the Maestro suite when asked.
//!
//! Usage:
//!   fs-sim                 # boot, build, install, launch, screenshot
//!   fs-sim --smoke         # and run the flows tagged `smoke`
//!   fs-sim --test          # and run the whole Maestro suite
//!   fs-sim --no-build      # reuse the existing app bundle
//!   fs-sim --clean         # clean the artefacts first
//!   fs-sim --device=<udid> # use a specific simulator

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use foodshare_orchestrator::Workspace;
use fs_app::build::{Configuration, Layout};
use fs_app::sim_flow::{self, SimulatorDevice};

#[derive(Parser, Debug)]
#[command(name = "fs-sim", version, about = "FoodShare simulator runner")]
struct Cli {
    /// The app repository. Defaults to the directory that holds
    /// `foodshare-app`.
    #[arg(long, global = true)]
    app_dir: Option<PathBuf>,

    /// The simulator to use.
    #[arg(long, global = true)]
    device: Option<String>,

    /// Clean the build artefacts first.
    #[arg(long)]
    clean: bool,

    /// Run the flows tagged `smoke`.
    #[arg(long)]
    smoke: bool,

    /// Run the whole Maestro suite.
    #[arg(long)]
    test: bool,

    /// Reuse the existing app bundle instead of rebuilding.
    #[arg(long)]
    no_build: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let workspace = match cli.app_dir.as_deref() {
        Some(app) => Workspace::with_app_dir(app),
        None => Workspace::discover()?,
    };
    let layout = Layout::new(workspace.app());

    println!("📱 =========================================================");
    println!("📱 FoodShare Native iOS Programmatic Runner");
    println!("📱 =========================================================");

    if let Ok(stats) = foodshare_orchestrator::disk::workspace_stats(&layout.app_dir).await {
        println!("💾 {} GB free ({}%)", stats.avail_gb, stats.avail_percent);
    }

    if cli.clean {
        let removed = fs_app::build::clean_artifacts(&layout);
        println!("🧹 cleaned {} build director(ies)", removed.len());
    }

    let device = sim_flow::resolve_or_boot(cli.device.as_deref())?;
    println!("📱 Target simulator: {} ({})", device.name, device.id);

    let bundle = layout.app_bundle(Configuration::Debug);
    let app_path = if cli.no_build && bundle.is_dir() {
        bundle
    } else {
        sim_flow::build_for_device(&layout, &device, cli.clean).await?
    };

    let pid = sim_flow::install_and_launch(&device, &app_path)?;
    println!(
        "✅ App launched{}",
        pid.map(|p| format!(" (pid {p})")).unwrap_or_default()
    );

    let screenshot = sim_flow::capture_screenshot(&device, &layout.app_dir, None)?;
    println!("📸 Screenshot captured: {}", screenshot.display());

    if cli.smoke || cli.test {
        println!("\n🧪 Running the Maestro suite against the target simulator...");
        let maestro_dir = foodshare_maestro::maestro_dir(&layout.app_dir);
        let mut args = Vec::new();
        if cli.smoke {
            args.push("--include-tags=smoke".to_string());
        }
        let outcome =
            foodshare_maestro::runner::run_test(&device_of(&device), &args, &maestro_dir, None)
                .await;
        if !outcome.passed {
            anyhow::bail!(
                "❌ Maestro failed: {}",
                outcome.error.unwrap_or_else(|| "unknown error".to_string())
            );
        }
        println!("🎉 Maestro E2E tests passed successfully!");
    }

    println!("🎉 Programmatic execution complete!");
    Ok(())
}

/// The device, as the Maestro runner names it.
fn device_of(device: &SimulatorDevice) -> foodshare_maestro::TargetDevice {
    foodshare_maestro::TargetDevice {
        platform: foodshare_maestro::Platform::Ios,
        id: device.id.clone(),
        name: device.name.clone(),
    }
}
