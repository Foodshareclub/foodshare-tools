//! fs-verify — the full cross-repository verification.
//!
//! The toolchain policy in every repository, the disk, the Rust
//! workspace, the web build, the mobile smoke tests and the backend edge
//! function tests.
//!
//! Usage:
//!   fs-verify                 # everything
//!   fs-verify --skip-mobile   # without the simulator smoke tests
//!   fs-verify --skip-node-guard
//!   fs-verify --json          # machine-readable output

use anyhow::Result;
use clap::{Parser, Subcommand};

use foodshare_orchestrator::{Verifier, Workspace};

#[derive(Parser, Debug)]
#[command(
    name = "fs-verify",
    version,
    about = "FoodShare full workspace verification"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Action>,

    /// The workspace root. Defaults to the directory that holds
    /// `foodshare-app`.
    #[arg(long, global = true)]
    root: Option<std::path::PathBuf>,

    /// Leave out the mobile smoke tests. They need a booted simulator.
    #[arg(long)]
    skip_mobile: bool,

    /// Leave out the toolchain policy check.
    #[arg(long)]
    skip_node_guard: bool,

    /// Print the results as JSON.
    #[arg(long)]
    json: bool,

    /// Where the run history is kept. Defaults to the state directory.
    #[arg(long)]
    state: Option<std::path::PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Action {
    /// The full verification.
    Verify,
    /// The toolchain policy check, in every repository.
    NodeCheck,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let workspace = match cli.root {
        Some(root) => Workspace::with_root(root),
        None => Workspace::discover()?,
    };

    let action = cli.command.unwrap_or(Action::Verify);

    if matches!(action, Action::NodeCheck) {
        let verifier = Verifier::new(&workspace).only_node_guard();
        // Ctrl-C stops the verification, and the build it is running, rather
        // than orphaning it.
        let cancelling = verifier.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                println!("\n🛑 cancelling the verification...");
                cancelling.cancel();
            }
        });

        let report = verifier.run(workspace.root()).await;
        for stage in &report.stages {
            let icon = if stage.success { "✅" } else { "❌" };
            println!("{icon} {}", stage.label);
            if let Some(error) = &stage.error {
                println!("   {error}");
            }
        }
        if !report.success {
            std::process::exit(1);
        }
        println!("\n✅ Every repository runs on Bun. No Node.js in the toolchain.");
        return Ok(());
    }

    let config = foodshare_orchestrator::plan::PlanConfig::load(workspace.root())?;
    let history = foodshare_orchestrator::history::History::load(
        cli.state
            .or_else(foodshare_orchestrator::history::default_path),
        foodshare_orchestrator::history::DEFAULT_LIMIT,
    )?;
    let mut verifier =
        Verifier::with_config(&workspace, &config).with_history(std::sync::Arc::new(history));
    if cli.skip_mobile {
        verifier = verifier.without_mobile();
    }
    if cli.skip_node_guard {
        verifier = verifier.without_node_guard();
    }

    if !cli.json {
        println!("🌟 =========================================================");
        println!("🌟 FoodShare Full Domain Verification");
        println!("🌟 =========================================================");
    }

    // Ctrl-C stops the verification, and the build it is running, rather
    // than orphaning it.
    let cancelling = verifier.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            println!("\n🛑 cancelling the verification...");
            cancelling.cancel();
        }
    });

    let report = verifier.run(workspace.root()).await;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        for stage in &report.stages {
            let icon = if stage.success { "✅" } else { "❌" };
            println!("{icon} {} ({}ms)", stage.label, stage.duration_ms);
            if let Some(error) = &stage.error {
                println!("   {error}");
            }
        }
        println!("\n==========================================");
        if report.success {
            println!("🎉 ALL DOMAIN SYSTEMS VERIFIED 100% GREEN!");
        } else {
            println!("❌ VERIFICATION FAILED after {}ms", report.duration_ms);
        }
        println!("==========================================\n");
    }

    if !report.success {
        std::process::exit(1);
    }
    Ok(())
}
