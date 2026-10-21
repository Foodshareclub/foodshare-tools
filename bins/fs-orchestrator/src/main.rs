//! fs-orchestrator — the Foodshare workspace orchestrator.
//!
//! One implementation of the workspace verification, with three
//! front-ends:
//!
//! - `fs-orchestrator serve` — the HTTP daemon, for a runner or a
//!   dashboard.
//! - `fs-orchestrator run` — the same plan from the command line.
//! - `foodshare-orchestrator` as a library — for the TUI.
//!
//! The routes and the probe contract match the backend's `job-worker`, so
//! one Prometheus covers both services.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use foodshare_orchestrator::{DomainKind, Options, Orchestrator, Workspace};

mod http;

use http::{AppState, SERVICE_NAME};

#[derive(Parser, Debug)]
#[command(
    name = "fs-orchestrator",
    version,
    about = "Foodshare workspace orchestrator"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// The address the daemon listens on.
    #[arg(long, default_value = "127.0.0.1:7878", global = true)]
    addr: String,

    /// The workspace root. Defaults to the directory that holds
    /// `foodshare-app`.
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    /// The token that guards the control routes. Also read from
    /// `FOODSHARE_ORCHESTRATOR_TOKEN`.
    #[arg(long, global = true, env = "FOODSHARE_ORCHESTRATOR_TOKEN")]
    token: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Serve the HTTP API.
    Serve,
    /// Run the verification plan without serving anything.
    Run {
        /// The domain to run: rust, wasm, web, backend, mobile, or all.
        #[arg(long, default_value = "all")]
        domain: String,
        /// Skip the WASM build.
        #[arg(long)]
        quick: bool,
        /// Run the web and backend domains one after another.
        #[arg(long)]
        sequential: bool,
    },
    /// Print the current status as JSON.
    Status,
    /// Prune the caches and report the free space.
    Disk {
        /// Prune even when there is enough room.
        #[arg(long)]
        force: bool,
        /// The free space to aim for, in gigabytes.
        #[arg(long, default_value_t = foodshare_orchestrator::disk::MIN_FREE_GB)]
        min_free_gb: f64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    foodshare_telemetry::init().ok();

    let workspace = match cli.root {
        Some(root) => Workspace::with_root(root),
        None => Workspace::discover()?,
    };

    let command = cli.command.unwrap_or(Command::Serve);
    match command {
        Command::Serve => serve(&workspace, &cli.addr, cli.token).await,
        Command::Run {
            domain,
            quick,
            sequential,
        } => {
            let orchestrator = Orchestrator::with_options(workspace, Options { quick, sequential });
            run(&orchestrator, &domain).await
        }
        Command::Status => {
            let orchestrator = Orchestrator::new(workspace);
            let snapshot = orchestrator.snapshot();
            println!("{}", serde_json::to_string_pretty(&snapshot)?);
            Ok(())
        }
        Command::Disk { force, min_free_gb } => heal_disk(&workspace, min_free_gb, force).await,
    }
}

/// Prune the caches and report the free space.
async fn heal_disk(workspace: &Workspace, min_free_gb: f64, force: bool) -> Result<()> {
    let root = workspace.root();
    let before = foodshare_orchestrator::disk::workspace_stats(root).await?;
    println!(
        "💾 Current free disk space: {} GB ({}% free)",
        before.avail_gb, before.avail_percent
    );

    let report = foodshare_orchestrator::disk::heal(root, min_free_gb, force).await?;
    if !report.performed {
        println!("✨ Nothing to clean.");
        return Ok(());
    }

    for action in &report.actions {
        println!("  ✓ {action}");
    }
    println!(
        "✨ Disk cleanup complete: Reclaimed ~{} GB. Available: {} GB.",
        report.freed_gb,
        before.avail_gb + report.freed_gb
    );
    Ok(())
}

/// Serve the HTTP API until the process is asked to stop.
async fn serve(workspace: &Workspace, addr: &str, token: Option<String>) -> Result<()> {
    let orchestrator = Orchestrator::new(workspace.clone());
    let guarded = token.is_some();
    let state = AppState::new(orchestrator).with_token(token);
    let app = http::router(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;
    tracing::info!(
        service = SERVICE_NAME,
        workspace = %workspace.root().display(),
        guarded,
        "serving"
    );
    println!("🦀 {SERVICE_NAME} listening on http://{bound}");
    println!("   workspace: {}", workspace.root().display());
    println!("   routes:    /healthz /readyz /metrics /status /run/{{domain}} /cancel");
    if guarded {
        println!("   the control routes need `Authorization: Bearer <token>`");
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Run a domain, or the whole plan, and report what happened.
async fn run(orchestrator: &Orchestrator, domain: &str) -> Result<()> {
    let started = std::time::Instant::now();
    println!("🦀 FoodShare Orchestrator — running `{domain}`");

    let (success, summary) = if domain.eq_ignore_ascii_case("all") {
        let outcome = orchestrator.run_plan().await?;
        let summary = outcome
            .domains
            .iter()
            .map(|d| {
                format!(
                    "{} {} ({}ms)",
                    d.kind.label(),
                    if d.success { "✅" } else { "❌" },
                    d.duration_ms
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        (outcome.success, summary)
    } else {
        let Some(kind) = DomainKind::from_name(&domain.to_ascii_lowercase()) else {
            anyhow::bail!("unknown domain `{domain}`");
        };
        let outcome = orchestrator.run_domain(kind).await?;
        let summary = format!(
            "{} {} ({}ms)",
            kind.label(),
            if outcome.success { "✅" } else { "❌" },
            outcome.duration_ms
        );
        (outcome.success, summary)
    };

    println!("{summary}");
    let elapsed = started.elapsed().as_secs_f32();
    if success {
        println!("🎉 all green in {elapsed:.1}s");
    } else {
        println!("❌ failed after {elapsed:.1}s");
        std::process::exit(1);
    }
    Ok(())
}

/// Stop on Ctrl-C.
async fn shutdown_signal() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::error!(error = %e, "failed to listen for Ctrl-C");
    }
    println!("\n🛑 shutting down");
}
