//! fs-auto — FoodShare headless auto-orchestrator
//!
//! Run-until-green, build, auto-close & clean.
//!
//! The orchestration lives in `foodshare-orchestrator`; this binary is the
//! command-line front-end over it. The HTTP daemon (`fs-orchestrator`) and
//! the TUI (`fs-tui`) are the other two.

use anyhow::Result;
use clap::Parser;

use foodshare_orchestrator::{Options, Orchestrator, Workspace, disk};

#[derive(Parser, Debug)]
#[command(
    name = "fs-auto",
    about = "FoodShare auto — run until green, build, close & clean"
)]
struct Cli {
    /// Keep the build artefacts after a successful run.
    #[arg(long)]
    keep: bool,
    /// How many times a failing step is retried.
    #[arg(long, default_value_t = 2)]
    retry: u32,
    /// Clean the build artefacts before the run.
    #[arg(long)]
    clean: bool,
    /// Skip the WASM build, on the assumption the artefacts are cached.
    #[arg(long)]
    quick: bool,
    /// Run the web and backend domains one after another.
    #[arg(long)]
    sequential: bool,

    /// Where the run history is kept. Defaults to the state directory.
    #[arg(long)]
    state: Option<std::path::PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let workspace = Workspace::discover()?;
    let root = workspace.root().to_path_buf();

    println!("🤖 =========================================================");
    println!("🤖 FoodShare Auto — run-until-green, build, auto-close & clean");
    println!("🤖 =========================================================");
    println!("   root: {}", root.display());
    println!(
        "   keep={}, retry={}, clean={}, quick={}, sequential={}",
        cli.keep, cli.retry, cli.clean, cli.quick, cli.sequential
    );

    // The disk is checked before the run, not after a failure: a build
    // that runs out of space halfway through wastes the whole run.
    match disk::workspace_stats(&root) {
        Ok(stats) => println!("💾 {} GB free ({}%)", stats.avail_gb, stats.avail_percent),
        Err(e) => tracing::warn!(error = %e, "could not read the disk statistics"),
    }

    if cli.clean {
        let removed = disk::clean_artifacts(&root);
        println!("🧹 cleaned {} artefact director(ies)", removed.len());
    }

    let history = foodshare_orchestrator::history::History::load(
        cli.state
            .or_else(foodshare_orchestrator::history::default_path),
        foodshare_orchestrator::history::DEFAULT_LIMIT,
    )?;
    let orchestrator = Orchestrator::from_workspace(
        workspace,
        Options {
            quick: cli.quick,
            sequential: cli.sequential,
        },
    )?
    .with_history(std::sync::Arc::new(history));

    let outcome = orchestrator.run_plan().await?;

    for domain in &outcome.domains {
        println!(
            "{} {} ({}ms)",
            domain.kind.label(),
            if domain.success { "✅" } else { "❌" },
            domain.duration_ms
        );
    }

    println!("==========================================");
    if outcome.success {
        println!("🎉 AUTO SUCCESS — all green in {}ms", outcome.duration_ms);
    } else {
        println!("❌ AUTO FAILED after {}ms", outcome.duration_ms);
    }
    println!("==========================================\n");

    if !cli.keep {
        let removed = disk::clean_artifacts(&root);
        println!("✨ Auto-closed & cleaned — repo neat ({removed:?})");
    } else {
        println!("📦 Keeping artifacts (--keep)");
    }

    if !outcome.success {
        std::process::exit(1);
    }
    Ok(())
}
