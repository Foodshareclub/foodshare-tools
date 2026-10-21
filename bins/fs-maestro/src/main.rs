//! fs-maestro — the Maestro E2E runner.
//!
//! Replaces the Bun runner that used to drive Maestro. The flows, the
//! syntax check, the single-device suite and the multi-device matrix all
//! live in `foodshare-maestro`.
//!
//! Usage:
//!   fs-maestro syntax            # check every flow in the suite
//!   fs-maestro smoke             # run the flows tagged `smoke`
//!   fs-maestro test              # run the whole suite
//!   fs-maestro flow <file>       # run one flow
//!   fs-maestro report [format]   # run the suite and write a report
//!   fs-maestro hierarchy         # dump the UI hierarchy
//!   fs-maestro matrix [tag]      # run the suite on every device at once
//!   fs-maestro check             # show the target device

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::process::Command as Process;

use foodshare_core::workspace::Workspace;
use foodshare_maestro::{device, flows, runner};

#[derive(Parser, Debug)]
#[command(name = "fs-maestro", version, about = "FoodShare Maestro E2E runner")]
struct Cli {
    #[command(subcommand)]
    command: Option<Action>,

    /// The app repository. Defaults to the directory that holds
    /// `foodshare-app`.
    #[arg(long, global = true)]
    app_dir: Option<PathBuf>,

    /// The device to run on. Defaults to the first booted simulator.
    #[arg(long, global = true)]
    device: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Action {
    /// Show the device the suite would run on.
    Check,
    /// Check the syntax of every flow in the suite.
    Syntax,
    /// Run the flows tagged `smoke`.
    Smoke,
    /// Run the flows tagged `regression`.
    Regression,
    /// Run the flows tagged `critical`.
    Critical,
    /// Run the flows tagged `a11y`.
    A11y,
    /// Run the whole suite.
    Test,
    /// Run one flow.
    Flow {
        /// The flow file.
        file: PathBuf,
    },
    /// Run the suite and write a report.
    Report {
        /// The report format: JUNIT, HTML or JSON.
        #[arg(default_value = "JUNIT")]
        format: String,
    },
    /// Dump the UI hierarchy of the target device.
    Hierarchy,
    /// Run the suite on every booted device at once.
    Matrix {
        /// A tag to filter by, or a flow file to run.
        target: Option<String>,
    },
}

/// The action a command maps to.
fn command_of(command: Option<Action>) -> Action {
    command.unwrap_or(Action::Test)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let workspace = match cli.app_dir.as_deref() {
        Some(app) => Workspace::with_app_dir(app),
        None => Workspace::discover()?,
    };
    let maestro_dir = foodshare_maestro::maestro_dir(&workspace.app());

    // The disk is checked before a suite: a full volume fails the run in
    // a way that looks like a test failure.
    if let Ok(stats) = foodshare_core::disk::workspace_stats(&workspace.app()) {
        tracing::info!(avail_gb = stats.avail_gb, "disk space checked");
    }

    let code = match command_of(cli.command) {
        Action::Check => check(),
        Action::Syntax => syntax(&maestro_dir).await,
        Action::Smoke => tagged(&maestro_dir, "smoke", cli.device.as_deref()).await,
        Action::Regression => tagged(&maestro_dir, "regression", cli.device.as_deref()).await,
        Action::Critical => tagged(&maestro_dir, "critical", cli.device.as_deref()).await,
        Action::A11y => tagged(&maestro_dir, "a11y", cli.device.as_deref()).await,
        Action::Test => suite(&maestro_dir, &[], cli.device.as_deref()).await,
        Action::Flow { file } => {
            // A flow path may be given relative to the suite directory.
            let flow = if file.exists() {
                file
            } else {
                maestro_dir.join(&file)
            };
            run_on(&maestro_dir, &flow, &[], cli.device.as_deref()).await
        }
        Action::Report { format } => report(&maestro_dir, &format, cli.device.as_deref()).await,
        Action::Hierarchy => hierarchy(cli.device.as_deref()).await,
        Action::Matrix { target } => matrix(&maestro_dir, target.as_deref()).await,
    };

    std::process::exit(code);
}

/// Show the device the suite would run on.
fn check() -> i32 {
    match device::resolve(None) {
        Some(target) => {
            println!("📱 Active target device: {} ({})", target.name, target.id);
            0
        }
        None => {
            println!("📱 Active target device: None booted (run `fs-sim` to launch one)");
            0
        }
    }
}

/// Check the syntax of every flow.
async fn syntax(maestro_dir: &Path) -> i32 {
    println!("🔍 Validating Maestro test flow syntax...");
    let found = flows::discover(maestro_dir);
    if found.is_empty() {
        println!("⚠️ No flows found in {}", maestro_dir.display());
        return 0;
    }

    let results = runner::check_syntax(&found).await;
    let mut failed = 0;
    for result in &results {
        if result.passed {
            println!("✓ {}", result.path.display());
        } else {
            println!("❌ Syntax error in {}", result.path.display());
            failed += 1;
        }
    }

    if failed > 0 {
        println!("❌ {failed} flow(s) failed syntax validation.");
        1
    } else {
        println!("✅ All Maestro flow files passed syntax validation!");
        0
    }
}

/// Run the flows with a tag.
async fn tagged(maestro_dir: &Path, tag: &str, device_id: Option<&str>) -> i32 {
    let args = vec![format!("--include-tags={tag}")];
    suite(maestro_dir, &args, device_id).await
}

/// Run the whole suite on one device.
async fn suite(maestro_dir: &Path, extra_args: &[String], device_id: Option<&str>) -> i32 {
    run_on(maestro_dir, maestro_dir, extra_args, device_id).await
}

/// Run Maestro on one device, against a target path.
async fn run_on(
    _maestro_dir: &Path,
    target_path: &Path,
    extra_args: &[String],
    device_id: Option<&str>,
) -> i32 {
    let target = match device::resolve(device_id) {
        Some(target) => target,
        None => {
            println!("⚠️ No booted simulator or connected device found.");
            return 1;
        }
    };

    println!("🧪 Running Maestro on {} ({})...", target.name, target.id);
    let outcome = runner::run_test(&target, extra_args, target_path, None).await;
    if outcome.passed {
        println!("🎉 Maestro passed in {}ms", outcome.duration_ms);
        0
    } else {
        println!(
            "❌ Maestro failed in {}ms: {}",
            outcome.duration_ms,
            outcome.error.unwrap_or_else(|| "unknown".to_string())
        );
        1
    }
}

/// Run the suite and write a report.
async fn report(maestro_dir: &Path, format: &str, device_id: Option<&str>) -> i32 {
    let format = format.to_uppercase();
    let reports_dir = foodshare_maestro::device::reports_dir(&maestro_dir.to_path_buf());
    let _ = tokio::fs::create_dir_all(&reports_dir).await;
    let extension = format.to_lowercase();
    let out_file = reports_dir.join(format!("report.{extension}"));

    let target = match device::resolve(device_id) {
        Some(target) => target,
        None => {
            println!("⚠️ No booted simulator or connected device found.");
            return 1;
        }
    };

    let args = vec![
        format!("--format={format}"),
        format!("--output={}", out_file.display()),
    ];
    println!("🧪 Running Maestro on {} ({})...", target.name, target.id);
    let outcome = runner::run_test(&target, &args, maestro_dir, None).await;
    println!("📊 Report generated at: {}", out_file.display());
    if outcome.passed { 0 } else { 1 }
}

/// Dump the UI hierarchy.
async fn hierarchy(device_id: Option<&str>) -> i32 {
    let mut command = Process::new("maestro");
    command.arg("hierarchy").stdin(Stdio::null());
    if let Some(id) = device_id {
        command.arg("--device").arg(id);
    }
    let status = command.status().await;
    match status {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            println!("❌ could not run maestro: {e}");
            1
        }
    }
}

/// Run the suite on every device at once.
async fn matrix(maestro_dir: &Path, target: Option<&str>) -> i32 {
    let devices = device::discover();
    if devices.is_empty() {
        println!("⚠️ No booted iOS Simulators or connected Android Emulators found.");
        println!("Defaulting to single-target Maestro execution...");
        return suite(maestro_dir, &[], None).await;
    }

    println!("Found {} active target device(s):", devices.len());
    for target in &devices {
        println!(
            "  • [{}] {} ({})",
            target.platform.name().to_uppercase(),
            target.name,
            target.id
        );
    }

    let mut extra_args = Vec::new();
    let mut target_path = maestro_dir.to_path_buf();
    if let Some(arg) = target {
        if foodshare_maestro::TAGS.contains(&arg) {
            extra_args.push(format!("--include-tags={arg}"));
        } else if Path::new(arg).exists() {
            target_path = PathBuf::from(arg);
        } else {
            extra_args.push(arg.to_string());
        }
    }

    let reports_dir = foodshare_maestro::device::reports_dir(&maestro_dir.to_path_buf());
    let started = std::time::Instant::now();
    let outcomes = runner::run_matrix(&devices, &extra_args, &target_path, &reports_dir).await;
    let total_ms = started.elapsed().as_millis();

    let passed = outcomes.iter().filter(|o| o.passed).count();
    let failed = outcomes.len() - passed;

    println!("\n==========================================");
    println!("        MAESTRO MATRIX TEST SUMMARY       ");
    println!("==========================================");
    println!(" Total Devices:  {}", outcomes.len());
    println!(" Passed:         {passed}");
    println!(" Failed:         {failed}");
    println!(" Total Duration: {}ms", total_ms);
    println!("==========================================");
    for outcome in &outcomes {
        let icon = if outcome.passed { "✅" } else { "❌" };
        println!(
            " {icon} [{}] {}: {}ms",
            outcome.device.platform.name().to_uppercase(),
            outcome.device.name,
            outcome.duration_ms
        );
        if let Some(log) = &outcome.log_file {
            println!("      log: {}", log.display());
        }
    }
    println!("==========================================\n");

    i32::from(failed > 0)
}
