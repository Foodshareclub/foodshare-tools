//! Foodshare iOS CLI
//!
//! Git hooks and development tools for Foodshare iOS.

use anyhow::Result;
use clap::{Parser, Subcommand};
use foodshare_cli::output::Status;
use foodshare_core::config::Config;
use foodshare_core::error::exit_codes;
use owo_colors::OwoColorize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "foodshare-ios")]
#[command(about = "Git hooks and development tools for Foodshare iOS")]
#[command(version)]
struct Cli {
    /// Config file path
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Increase output verbosity
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Format Swift code with enterprise-grade safety features
    Format {
        /// Files to format
        #[arg(trailing_var_arg = true)]
        files: Vec<PathBuf>,
        /// Check only, don't modify (legacy flag, use --preview)
        #[arg(long)]
        check: bool,
        /// Format only staged files
        #[arg(long)]
        staged: bool,
        /// Preview mode: show what would change without modifying files
        #[arg(long)]
        preview: bool,
        /// Create a stash backup before formatting (default: true)
        #[arg(long, default_value = "true")]
        backup: bool,
        /// Disable backup (equivalent to --backup=false)
        #[arg(long)]
        no_backup: bool,
        /// Show detailed diff of changes
        #[arg(long, default_value = "true")]
        show_diff: bool,
        /// Enable audit logging
        #[arg(long, default_value = "true")]
        audit: bool,
        /// Create a snapshot before formatting (Code Protection System, default: true)
        #[arg(long, default_value = "true")]
        snapshot: bool,
        /// Disable snapshot creation
        #[arg(long)]
        no_snapshot: bool,
    },

    /// Lint Swift code
    Lint {
        /// Files to lint
        #[arg(trailing_var_arg = true)]
        files: Vec<PathBuf>,
        /// Enable strict mode
        #[arg(long)]
        strict: bool,
        /// Auto-fix violations
        #[arg(long)]
        fix: bool,
    },

    /// Validate commit message
    #[command(name = "commit-msg")]
    CommitMsg {
        /// Path to commit message file
        file: PathBuf,
    },

    /// Scan for secrets
    Secrets {
        /// Check all files
        #[arg(long)]
        all: bool,
    },

    /// Check migrations status
    Migrations {
        /// Migrations directory
        #[arg(long, default_value = "supabase/migrations")]
        dir: PathBuf,
    },

    /// Build project
    Build {
        /// Build configuration
        #[arg(long, default_value = "debug")]
        configuration: String,
        /// Clean before building
        #[arg(long)]
        clean: bool,
    },

    /// Run tests
    Test {
        /// Enable coverage
        #[arg(long)]
        coverage: bool,
    },

    /// Build, install, and run the app on simulator
    Run {
        /// Clean before building
        #[arg(long)]
        clean: bool,
        /// Stream app logs after launch
        #[arg(long)]
        logs: bool,
        /// Use release configuration
        #[arg(long)]
        release: bool,
        /// Device name or UDID
        #[arg(long)]
        device: Option<String>,
    },

    /// List simulators
    Simulator {
        /// Action: list, boot, shutdown
        action: String,
        /// Device name or UDID
        #[arg(long)]
        device: Option<String>,
    },

    /// Diagnose environment
    Doctor {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Xcode project management
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },

    /// Verify setup
    Verify,

    /// Pre-push checks - validates build, lint, and tests before push
    #[command(name = "pre-push")]
    PrePush {
        /// Remote name
        remote: Option<String>,
        /// Remote URL
        url: Option<String>,
        /// Fail fast on first error (default: true)
        #[arg(long, default_value = "true")]
        fail_fast: bool,
        /// Use release build for validation
        #[arg(long)]
        release: bool,
        /// Quick mode: skip optional checks (tests)
        #[arg(long)]
        quick: bool,
        /// Skip specific checks (comma-separated: lint,build,test)
        #[arg(long, value_delimiter = ',')]
        skip: Vec<String>,
        /// Show detailed output for pre-push checks
        #[arg(long)]
        detailed: bool,
    },

    /// Manage Swift package dependencies
    Deps {
        #[command(subcommand)]
        action: DepsAction,
    },

    /// Code protection system - snapshots, recovery, and safety guards
    Protect {
        #[command(subcommand)]
        action: ProtectAction,
    },
}

#[derive(Subcommand)]
enum ProtectAction {
    /// List all code snapshots
    List {
        /// Maximum number of snapshots to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Create a manual snapshot of current state
    Snapshot {
        /// Description of why this snapshot was created
        #[arg(long, default_value = "Manual snapshot")]
        description: String,
        /// Specific files to snapshot (default: all modified)
        #[arg(trailing_var_arg = true)]
        files: Vec<PathBuf>,
    },

    /// Restore files from a snapshot
    Restore {
        /// Restore from the latest snapshot
        #[arg(long)]
        latest: bool,
        /// Snapshot ID to restore from
        #[arg(long)]
        snapshot: Option<String>,
        /// Specific files to restore (default: all files in snapshot)
        #[arg(long)]
        file: Option<PathBuf>,
        /// Preview what would be restored without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Show what would be committed (Commit Guard)
    #[command(name = "commit-guard")]
    CommitGuard,

    /// Show what would be pushed (Push Guard)
    #[command(name = "push-guard")]
    PushGuard {
        /// Remote name (default: origin)
        #[arg(long, default_value = "origin")]
        remote: String,
        /// Branch name (default: current branch)
        #[arg(long)]
        branch: Option<String>,
    },

    /// Verify build after changes (Build Verification)
    #[command(name = "verify-build")]
    VerifyBuild {
        /// Quick syntax check only
        #[arg(long)]
        quick: bool,
    },

    /// Show operation history
    History {
        /// Number of recent operations to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Show protection status and configuration
    Status,
}

#[derive(Subcommand)]
enum DepsAction {
    /// Resolve package dependencies
    Resolve {
        /// Package directory (default: current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Update package dependencies to latest versions
    Update {
        /// Package directory (default: current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    /// Show project status (missing files, broken refs, duplicates)
    Status {
        /// Path to .xcodeproj
        #[arg(long, default_value = "FoodShare.xcodeproj")]
        project: PathBuf,
        /// Target name
        #[arg(long, default_value = "FoodShare")]
        target: String,
        /// Source directory
        #[arg(long, default_value = "FoodShare")]
        source_dir: String,
    },
    /// Find missing files (on disk but not in build phase)
    Missing {
        /// Path to .xcodeproj
        #[arg(long, default_value = "FoodShare.xcodeproj")]
        project: PathBuf,
        /// Target name
        #[arg(long, default_value = "FoodShare")]
        target: String,
        /// Source directory
        #[arg(long, default_value = "FoodShare")]
        source_dir: String,
    },
    /// Find broken references (in project but file doesn't exist)
    Broken {
        /// Path to .xcodeproj
        #[arg(long, default_value = "FoodShare.xcodeproj")]
        project: PathBuf,
    },
    /// Find duplicate build file references
    Duplicates {
        /// Path to .xcodeproj
        #[arg(long, default_value = "FoodShare.xcodeproj")]
        project: PathBuf,
        /// Target name
        #[arg(long, default_value = "FoodShare")]
        target: String,
    },
    /// Add files to the Xcode project
    Add {
        /// Files to add to the project
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Path to .xcodeproj
        #[arg(long, default_value = "FoodShare.xcodeproj")]
        project: PathBuf,
        /// Target name to add source files to
        #[arg(long, default_value = "FoodShare")]
        target: String,
        /// Group path (e.g., "FoodShare/Core/Design")
        #[arg(long)]
        group: Option<String>,
        /// Preview changes without modifying the project
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.no_color {
        owo_colors::set_override(false);
    }

    let config = Config::load(cli.config.as_deref().map(|p| p.to_str().unwrap()))?;

    let exit_code = match cli.command {
        Commands::Format { files, check, staged, preview, backup, no_backup, show_diff, audit, snapshot, no_snapshot } => {
            run_format(&files, check || preview, staged, preview, backup && !no_backup, show_diff, audit, snapshot && !no_snapshot)
        }
        Commands::Lint { files, strict, fix } => {
            run_lint(&files, strict, fix)
        }
        Commands::CommitMsg { file } => {
            run_commit_msg(&file, &config)
        }
        Commands::Secrets { all } => {
            run_secrets(all, &config)
        }
        Commands::Migrations { dir } => {
            run_migrations(&dir)
        }
        Commands::Build { configuration, clean } => {
            run_build(&configuration, clean)
        }
        Commands::Test { coverage } => {
            run_test(coverage)
        }
        Commands::Run { clean, logs, release, device } => {
            run_app(clean, logs, release, device.as_deref())
        }
        Commands::Simulator { action, device } => {
            run_simulator(&action, device.as_deref())
        }
        Commands::Doctor { json } => {
            run_doctor(json)
        }
        Commands::Project { action } => {
            run_project(action)
        }
        Commands::Verify => {
            run_verify()
        }
        Commands::PrePush { remote, url, fail_fast, release, quick, skip, detailed } => {
            run_pre_push(remote.as_deref(), url.as_deref(), fail_fast, release, quick, skip, detailed)
        }
        Commands::Deps { action } => {
            run_deps(action)
        }
        Commands::Protect { action } => {
            run_protect(action)
        }
    };

    std::process::exit(exit_code);
}

fn run_format(files: &[PathBuf], check: bool, staged: bool, preview: bool, backup: bool, show_diff: bool, audit: bool, create_snapshot: bool) -> i32 {
    use foodshare_ios::hooks::{SafeFormat, SafeFormatConfig, print_format_summary};
    use foodshare_ios::swift_tools;

    if !swift_tools::has_swiftformat() {
        Status::error("swiftformat not found. Install with: brew install swiftformat");
        return exit_codes::FAILURE;
    }

    // Determine target files
    let target_files = if staged {
        match foodshare_core::git::GitRepo::open_current()
            .and_then(|r| r.staged_swift_files())
        {
            Ok(f) => f,
            Err(e) => {
                Status::error(&format!("Failed to get staged files: {}", e));
                return exit_codes::FAILURE;
            }
        }
    } else if files.is_empty() {
        // Scan for Swift files in FoodShare directory
        foodshare_core::file_scanner::scan_swift_files(std::path::Path::new("FoodShare"))
            .unwrap_or_default()
    } else {
        files.to_vec()
    };

    if target_files.is_empty() {
        Status::info("No Swift files to format");
        return exit_codes::SUCCESS;
    }

    // Legacy check mode - just run swiftformat --lint
    if check && !preview {
        return run_legacy_format_check(&target_files);
    }

    // Enterprise-grade safe format
    let config = SafeFormatConfig {
        preview,
        backup,
        show_diff,
        audit,
        create_snapshot,
        ..Default::default()
    };

    let safe_format = match SafeFormat::new(config) {
        Ok(sf) => sf,
        Err(e) => {
            Status::error(&format!("Failed to initialize SafeFormat: {}", e));
            return exit_codes::FAILURE;
        }
    };

    println!();
    println!("{}", "Safe Format".bold());
    println!("{}", "═".repeat(50));
    println!();

    if preview {
        println!("  {} Preview mode enabled - no files will be modified", "👁".yellow());
    }
    if create_snapshot {
        println!("  {} Snapshot protection enabled", "📸".green());
    }
    if backup {
        println!("  {} Stash backup enabled", "💾".green());
    }
    println!();

    match safe_format.format(&target_files) {
        Ok(result) => {
            print_format_summary(&result);

            if result.failed_files.is_empty() {
                exit_codes::SUCCESS
            } else {
                exit_codes::FAILURE
            }
        }
        Err(e) => {
            Status::error(&format!("Format error: {}", e));
            exit_codes::FAILURE
        }
    }
}

/// Legacy format check (swiftformat --lint)
fn run_legacy_format_check(files: &[PathBuf]) -> i32 {
    use foodshare_ios::swift_tools;

    Status::info("Running format check (legacy mode)...");

    for file in files {
        if file.is_dir() {
            match swift_tools::format_directory(file, true) {
                Ok(result) => {
                    if !result.success {
                        Status::error("Format check failed - files need formatting");
                        return exit_codes::FAILURE;
                    }
                }
                Err(e) => {
                    Status::error(&format!("Format check error: {}", e));
                    return exit_codes::FAILURE;
                }
            }
        }
    }

    Status::success("Format check passed");
    exit_codes::SUCCESS
}

fn run_lint(files: &[PathBuf], strict: bool, fix: bool) -> i32 {
    use foodshare_ios::swift_tools;

    if !swift_tools::has_swiftlint() {
        Status::error("swiftlint not found. Install with: brew install swiftlint");
        return exit_codes::FAILURE;
    }

    let target_dir = if files.is_empty() {
        PathBuf::from("FoodShare")
    } else {
        files[0].clone()
    };

    match swift_tools::lint_directory(&target_dir, strict, fix) {
        Ok(result) => {
            if result.success {
                Status::success("Lint complete");
                exit_codes::SUCCESS
            } else {
                Status::error("Lint found issues");
                println!("{}", result.stdout);
                exit_codes::FAILURE
            }
        }
        Err(e) => {
            Status::error(&format!("Lint error: {}", e));
            exit_codes::FAILURE
        }
    }
}

fn run_commit_msg(file: &PathBuf, config: &Config) -> i32 {
    use foodshare_hooks::commit_msg;

    match commit_msg::validate_commit_message(file, &config.schema.commit_msg) {
        Ok(result) => {
            if result.valid {
                Status::success(result.message.as_deref().unwrap_or("Valid"));
            } else {
                commit_msg::print_error(
                    &std::fs::read_to_string(file).unwrap_or_default(),
                    &config.schema.commit_msg.types,
                );
            }
            result.exit_code
        }
        Err(e) => {
            Status::error(&format!("Validation error: {}", e));
            exit_codes::FAILURE
        }
    }
}

fn run_secrets(all: bool, config: &Config) -> i32 {
    use foodshare_hooks::secrets;

    let files = if all {
        foodshare_core::file_scanner::scan_swift_files(std::path::Path::new("."))
            .unwrap_or_default()
    } else {
        foodshare_core::git::GitRepo::open_current()
            .and_then(|r| r.staged_files())
            .unwrap_or_default()
    };

    match secrets::scan_files(&files, &config.schema.secrets) {
        Ok(matches) => secrets::print_results(&matches),
        Err(e) => {
            Status::error(&format!("Scan error: {}", e));
            exit_codes::FAILURE
        }
    }
}

fn run_migrations(dir: &PathBuf) -> i32 {
    use foodshare_hooks::migrations;

    match migrations::check_migrations(dir, true, true) {
        Ok(check) => migrations::print_results(&check),
        Err(e) => {
            Status::error(&format!("Migration check error: {}", e));
            exit_codes::FAILURE
        }
    }
}

fn run_build(configuration: &str, clean: bool) -> i32 {
    use foodshare_ios::xcode;

    if !xcode::is_xcode_available() {
        Status::error("Xcode not found");
        return exit_codes::FAILURE;
    }

    Status::info(&format!("Building {} configuration...", configuration));

    match xcode::build(
        "FoodShare",
        configuration,
        "platform=iOS Simulator,name=iPhone 17 Pro Max",
        clean,
    ) {
        Ok(result) => {
            if result.success {
                Status::success("Build succeeded");
                exit_codes::SUCCESS
            } else {
                Status::error("Build failed");
                eprintln!("{}", result.stderr);
                exit_codes::FAILURE
            }
        }
        Err(e) => {
            Status::error(&format!("Build error: {}", e));
            exit_codes::FAILURE
        }
    }
}

fn run_test(coverage: bool) -> i32 {
    use foodshare_ios::xcode;

    Status::info("Running tests...");

    match xcode::test(
        "FoodShare",
        "platform=iOS Simulator,name=iPhone 17 Pro Max",
        coverage,
    ) {
        Ok(result) => {
            if result.success {
                Status::success("Tests passed");
                exit_codes::SUCCESS
            } else {
                Status::error("Tests failed");
                eprintln!("{}", result.stderr);
                exit_codes::FAILURE
            }
        }
        Err(e) => {
            Status::error(&format!("Test error: {}", e));
            exit_codes::FAILURE
        }
    }
}

fn run_app(clean: bool, logs: bool, release: bool, device: Option<&str>) -> i32 {
    use foodshare_ios::{simulator, xcode};

    let device_name = device.unwrap_or("iPhone 17 Pro Max");
    let configuration = if release { "Release" } else { "Debug" };
    let destination = format!("platform=iOS Simulator,name={}", device_name);

    // Step 1: Build
    Status::info(&format!("Building {} configuration...", configuration));
    match xcode::build("FoodShare", configuration, &destination, clean) {
        Ok(result) => {
            if !result.success {
                Status::error("Build failed");
                eprintln!("{}", result.stderr);
                return exit_codes::FAILURE;
            }
            Status::success("Build succeeded");
        }
        Err(e) => {
            Status::error(&format!("Build error: {}", e));
            return exit_codes::FAILURE;
        }
    }

    // Step 2: Get booted device or boot one
    let device_udid = match simulator::get_booted_device() {
        Ok(Some(udid)) => udid,
        Ok(None) => {
            Status::info(&format!("Booting {}...", device_name));
            if let Err(e) = simulator::boot(device_name) {
                Status::error(&format!("Failed to boot simulator: {}", e));
                return exit_codes::FAILURE;
            }
            // Wait a moment for boot
            std::thread::sleep(std::time::Duration::from_secs(2));
            match simulator::get_booted_device() {
                Ok(Some(udid)) => udid,
                _ => {
                    Status::error("Failed to get booted device");
                    return exit_codes::FAILURE;
                }
            }
        }
        Err(e) => {
            Status::error(&format!("Failed to check simulator status: {}", e));
            return exit_codes::FAILURE;
        }
    };

    // Step 3: Find the built app
    let derived_data = std::env::current_dir()
        .unwrap_or_default()
        .join("build")
        .join("Build")
        .join("Products")
        .join(format!("{}-iphonesimulator", configuration))
        .join("FoodShare.app");

    if !derived_data.exists() {
        Status::error(&format!("App not found at: {}", derived_data.display()));
        return exit_codes::FAILURE;
    }

    // Step 4: Install
    Status::info("Installing app...");
    match simulator::install_app(&device_udid, derived_data.to_str().unwrap()) {
        Ok(result) => {
            if !result.success {
                Status::error("Install failed");
                eprintln!("{}", result.stderr);
                return exit_codes::FAILURE;
            }
            Status::success("App installed");
        }
        Err(e) => {
            Status::error(&format!("Install error: {}", e));
            return exit_codes::FAILURE;
        }
    }

    // Step 5: Launch
    Status::info("Launching app...");
    match simulator::launch_app(&device_udid, "com.flutterflow.foodshare") {
        Ok(result) => {
            if !result.success {
                Status::error("Launch failed");
                eprintln!("{}", result.stderr);
                return exit_codes::FAILURE;
            }
            Status::success("App launched");
        }
        Err(e) => {
            Status::error(&format!("Launch error: {}", e));
            return exit_codes::FAILURE;
        }
    }

    // Step 6: Stream logs if requested
    if logs {
        Status::info("Streaming logs (Ctrl+C to stop)...");
        use std::process::{Command, Stdio};
        let mut child = Command::new("xcrun")
            .args([
                "simctl",
                "spawn",
                "booted",
                "log",
                "stream",
                "--predicate",
                "subsystem == \"com.flutterflow.foodshare\"",
                "--level",
                "info",
            ])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Failed to spawn log stream");
        let _ = child.wait();
    }

    exit_codes::SUCCESS
}

fn run_simulator(action: &str, device: Option<&str>) -> i32 {
    use foodshare_ios::simulator;

    match action {
        "list" => {
            match simulator::list_devices() {
                Ok(devices) => {
                    println!("Available Simulators:");
                    for d in devices.iter().filter(|d| d.is_available) {
                        let status = if d.state == "Booted" { "🟢" } else { "⚪" };
                        println!("  {} {} ({})", status, d.name, d.runtime);
                    }
                    exit_codes::SUCCESS
                }
                Err(e) => {
                    Status::error(&format!("Failed to list simulators: {}", e));
                    exit_codes::FAILURE
                }
            }
        }
        "boot" => {
            let device_name = device.unwrap_or("iPhone 17 Pro Max");
            match simulator::boot(device_name) {
                Ok(_) => {
                    Status::success(&format!("Booted {}", device_name));
                    exit_codes::SUCCESS
                }
                Err(e) => {
                    Status::error(&format!("Failed to boot: {}", e));
                    exit_codes::FAILURE
                }
            }
        }
        "shutdown" => {
            match simulator::shutdown_all() {
                Ok(_) => {
                    Status::success("Shutdown all simulators");
                    exit_codes::SUCCESS
                }
                Err(e) => {
                    Status::error(&format!("Failed to shutdown: {}", e));
                    exit_codes::FAILURE
                }
            }
        }
        _ => {
            Status::error(&format!("Unknown action: {}", action));
            exit_codes::FAILURE
        }
    }
}

fn run_doctor(json: bool) -> i32 {
    use foodshare_ios::{swift_tools, xcode};

    if json {
        // TODO: JSON output
        Status::info("JSON output not yet implemented");
        return exit_codes::SUCCESS;
    }

    println!("Environment Check");
    println!();

    // Xcode
    if xcode::is_xcode_available() {
        if let Ok(version) = xcode::xcode_version() {
            Status::success(&format!("Xcode: {}", version));
        }
    } else {
        Status::error("Xcode: not found");
    }

    // Swift
    if let Ok(version) = swift_tools::swift_version() {
        Status::success(&format!("Swift: {}", version));
    } else {
        Status::error("Swift: not found");
    }

    // swiftformat
    if swift_tools::has_swiftformat() {
        Status::success("swiftformat: installed");
    } else {
        Status::warning("swiftformat: not found (optional)");
    }

    // swiftlint
    if swift_tools::has_swiftlint() {
        Status::success("swiftlint: installed");
    } else {
        Status::warning("swiftlint: not found (optional)");
    }

    exit_codes::SUCCESS
}

fn run_verify() -> i32 {
    Status::info("Verifying setup...");

    // Check lefthook
    if foodshare_core::process::command_exists("lefthook") {
        Status::success("lefthook: installed");
    } else {
        Status::error("lefthook: not found");
        return exit_codes::FAILURE;
    }

    // Check git hooks
    let hooks_dir = std::path::Path::new(".git/hooks");
    if hooks_dir.exists() {
        Status::success("Git hooks directory exists");
    } else {
        Status::warning("Git hooks directory not found");
    }

    Status::success("Setup verified");
    exit_codes::SUCCESS
}


fn run_pre_push(
    _remote: Option<&str>,
    _url: Option<&str>,
    fail_fast: bool,
    release: bool,
    quick: bool,
    skip: Vec<String>,
    detailed: bool,
) -> i32 {
    use foodshare_ios::hooks::{run_pre_push_checks, print_pre_push_summary, PrePushConfig};

    // Check for quick mode environment variable
    let quick_mode = quick || std::env::var("FOODSHARE_QUICK_MODE").is_ok();

    let config = PrePushConfig {
        fail_fast,
        release,
        quick_mode,
        skip_checks: skip,
    };

    if detailed {
        println!("Pre-push configuration:");
        println!("  fail_fast: {}", config.fail_fast);
        println!("  release: {}", config.release);
        println!("  quick_mode: {}", config.quick_mode);
        println!("  skip_checks: {:?}", config.skip_checks);
        println!();
    }

    let results = run_pre_push_checks(&config);
    print_pre_push_summary(&results)
}

fn run_deps(action: DepsAction) -> i32 {
    use foodshare_ios::swift_tools;

    // Extract path and determine action type
    let (path, is_update) = match &action {
        DepsAction::Resolve { path } => (path, false),
        DepsAction::Update { path } => (path, true),
    };

    // Check for Package.swift before proceeding
    let package_swift = path.join("Package.swift");
    if !package_swift.exists() {
        Status::error(&format!(
            "No Package.swift found in {}",
            if path.as_os_str() == "." {
                "current directory".to_string()
            } else {
                path.display().to_string()
            }
        ));
        Status::info("Run this command from a Swift package directory");
        return exit_codes::FAILURE;
    }

    // Check for Package.resolved (optional warning)
    let package_resolved = path.join("Package.resolved");
    if !package_resolved.exists() && !is_update {
        Status::warning("No Package.resolved found - dependencies are not locked");
    }

    if is_update {
        Status::info("Updating Swift package dependencies...");
        Status::warning("This will modify Package.resolved");
        match swift_tools::update_dependencies(path) {
            Ok(result) => {
                if result.success {
                    Status::success("Dependencies updated");
                    exit_codes::SUCCESS
                } else {
                    Status::error("Failed to update dependencies");
                    eprintln!("{}", result.combined_output());
                    exit_codes::FAILURE
                }
            }
            Err(e) => {
                Status::error(&format!("Update error: {}", e));
                exit_codes::FAILURE
            }
        }
    } else {
        Status::info("Resolving Swift package dependencies...");
        match swift_tools::resolve_dependencies(path) {
            Ok(result) => {
                if result.success {
                    Status::success("Dependencies resolved");
                    exit_codes::SUCCESS
                } else {
                    Status::error("Failed to resolve dependencies");
                    eprintln!("{}", result.combined_output());
                    exit_codes::FAILURE
                }
            }
            Err(e) => {
                Status::error(&format!("Resolve error: {}", e));
                exit_codes::FAILURE
            }
        }
    }
}

fn run_project(action: ProjectAction) -> i32 {
    use foodshare_ios::xcodeproj::XcodeProject;
    use owo_colors::OwoColorize;

    match action {
        ProjectAction::Status { project, target, source_dir } => {
            Status::info(&format!("Analyzing {}...", project.display()));

            match XcodeProject::open(&project) {
                Ok(proj) => {
                    match proj.status(&target, &source_dir) {
                        Ok(status) => {
                            println!();
                            status.print();

                            if status.is_clean() {
                                println!();
                                Status::success("Project is clean!");
                            } else {
                                println!();
                                Status::warning("Project has issues. Run subcommands for details.");
                            }
                            exit_codes::SUCCESS
                        }
                        Err(e) => {
                            Status::error(&format!("Analysis failed: {}", e));
                            exit_codes::FAILURE
                        }
                    }
                }
                Err(e) => {
                    Status::error(&format!("Failed to open project: {}", e));
                    exit_codes::FAILURE
                }
            }
        }

        ProjectAction::Missing { project, target, source_dir } => {
            match XcodeProject::open(&project) {
                Ok(proj) => {
                    match proj.find_missing_files(&target, &source_dir) {
                        Ok(missing) => {
                            if missing.is_empty() {
                                Status::success("No missing files found");
                            } else {
                                println!("{}", "Missing files (on disk but not in build phase):".bold());
                                println!();
                                for path in &missing {
                                    println!("  {} {}", "+".green(), path.display());
                                }
                                println!();
                                println!("Total: {} file(s)", missing.len());
                            }
                            exit_codes::SUCCESS
                        }
                        Err(e) => {
                            Status::error(&format!("Scan failed: {}", e));
                            exit_codes::FAILURE
                        }
                    }
                }
                Err(e) => {
                    Status::error(&format!("Failed to open project: {}", e));
                    exit_codes::FAILURE
                }
            }
        }

        ProjectAction::Broken { project } => {
            match XcodeProject::open(&project) {
                Ok(proj) => {
                    let broken = proj.find_broken_references();
                    if broken.is_empty() {
                        Status::success("No broken references found");
                    } else {
                        println!("{}", "Broken references (in project but file doesn't exist):".bold());
                        println!();
                        for fr in &broken {
                            println!("  {} {}", "✗".red(), fr.path);
                        }
                        println!();
                        println!("Total: {} reference(s)", broken.len());
                    }
                    exit_codes::SUCCESS
                }
                Err(e) => {
                    Status::error(&format!("Failed to open project: {}", e));
                    exit_codes::FAILURE
                }
            }
        }

        ProjectAction::Duplicates { project, target } => {
            match XcodeProject::open(&project) {
                Ok(proj) => {
                    let duplicates = proj.find_duplicate_build_files(&target);
                    if duplicates.is_empty() {
                        Status::success("No duplicate references found");
                    } else {
                        println!("{}", "Duplicate build file references:".bold());
                        println!();
                        for (file_ref_id, build_files) in &duplicates {
                            println!("  File ref {}: {} duplicates", file_ref_id, build_files.len());
                        }
                        println!();
                        println!("Total: {} file(s) with duplicates", duplicates.len());
                    }
                    exit_codes::SUCCESS
                }
                Err(e) => {
                    Status::error(&format!("Failed to open project: {}", e));
                    exit_codes::FAILURE
                }
            }
        }

        ProjectAction::Add { files, project, target, group, dry_run } => {
            if dry_run {
                Status::info("Dry run mode - no changes will be made");
            }

            match XcodeProject::open(&project) {
                Ok(mut proj) => {
                    let mut added = 0;
                    let mut skipped = 0;
                    let mut failed = 0;

                    for file in &files {
                        match proj.add_file(file, &target, group.as_deref()) {
                            Ok(result) => {
                                if result.already_exists {
                                    println!("  {} {} (already in project)", "~".yellow(), file.display());
                                    skipped += 1;
                                } else {
                                    println!("  {} {}", "+".green(), file.display());
                                    added += 1;
                                }
                            }
                            Err(e) => {
                                println!("  {} {} - {}", "✗".red(), file.display(), e);
                                failed += 1;
                            }
                        }
                    }

                    println!();
                    println!("Added: {}, Skipped: {}, Failed: {}", added, skipped, failed);

                    if !dry_run && added > 0 {
                        match proj.save() {
                            Ok(()) => {
                                Status::success("Project saved (backup created at project.pbxproj.backup)");
                            }
                            Err(e) => {
                                Status::error(&format!("Failed to save project: {}", e));
                                return exit_codes::FAILURE;
                            }
                        }
                    } else if dry_run && added > 0 {
                        Status::info("Run without --dry-run to apply changes");
                    }

                    if failed > 0 {
                        exit_codes::FAILURE
                    } else {
                        exit_codes::SUCCESS
                    }
                }
                Err(e) => {
                    Status::error(&format!("Failed to open project: {}", e));
                    exit_codes::FAILURE
                }
            }
        }
    }
}

// ============================================================================
// CODE PROTECTION COMMANDS
// ============================================================================

fn run_protect(action: ProtectAction) -> i32 {
    use foodshare_ios::code_protection::{
        CommitGuard, OperationHistory, ProtectionConfig, PushGuard, SnapshotManager,
        SnapshotTrigger, print_pending_commit, print_pending_push, print_restore_result,
        print_snapshot_list, verify_build,
    };

    let config = ProtectionConfig::default();

    match action {
        ProtectAction::List { limit } => {
            let manager = match SnapshotManager::new(config) {
                Ok(m) => m,
                Err(e) => {
                    Status::error(&format!("Failed to initialize snapshot manager: {}", e));
                    return exit_codes::FAILURE;
                }
            };

            match manager.list_snapshots() {
                Ok(snapshots) => {
                    let limited: Vec<_> = snapshots.into_iter().take(limit).collect();
                    print_snapshot_list(&limited);
                    exit_codes::SUCCESS
                }
                Err(e) => {
                    Status::error(&format!("Failed to list snapshots: {}", e));
                    exit_codes::FAILURE
                }
            }
        }

        ProtectAction::Snapshot { description, files } => {
            let manager = match SnapshotManager::new(config) {
                Ok(m) => m,
                Err(e) => {
                    Status::error(&format!("Failed to initialize snapshot manager: {}", e));
                    return exit_codes::FAILURE;
                }
            };

            // Get files to snapshot
            let target_files = if files.is_empty() {
                // Snapshot all modified files
                match foodshare_core::git::GitRepo::open_current()
                    .and_then(|r| r.uncommitted_files())
                {
                    Ok(f) => f,
                    Err(e) => {
                        Status::error(&format!("Failed to get modified files: {}", e));
                        return exit_codes::FAILURE;
                    }
                }
            } else {
                files
            };

            if target_files.is_empty() {
                Status::info("No files to snapshot");
                return exit_codes::SUCCESS;
            }

            println!();
            println!("{}", "Creating snapshot...".bold());

            match manager.create_snapshot(&target_files, SnapshotTrigger::Manual, &description) {
                Ok(snapshot) => {
                    println!();
                    Status::success(&format!(
                        "Snapshot created: {} ({} files)",
                        snapshot.id,
                        snapshot.files.len()
                    ));
                    println!();
                    println!("  Recovery command:");
                    println!("    {} protect restore --snapshot {}", "foodshare-ios".cyan(), snapshot.id);
                    println!();
                    exit_codes::SUCCESS
                }
                Err(e) => {
                    Status::error(&format!("Failed to create snapshot: {}", e));
                    exit_codes::FAILURE
                }
            }
        }

        ProtectAction::Restore { latest, snapshot, file, dry_run } => {
            let manager = match SnapshotManager::new(config) {
                Ok(m) => m,
                Err(e) => {
                    Status::error(&format!("Failed to initialize snapshot manager: {}", e));
                    return exit_codes::FAILURE;
                }
            };

            // Get the snapshot to restore from
            let snap = if latest {
                match manager.get_latest_snapshot() {
                    Ok(Some(s)) => s,
                    Ok(None) => {
                        Status::error("No snapshots found");
                        return exit_codes::FAILURE;
                    }
                    Err(e) => {
                        Status::error(&format!("Failed to get latest snapshot: {}", e));
                        return exit_codes::FAILURE;
                    }
                }
            } else if let Some(id) = snapshot {
                match manager.get_snapshot(&id) {
                    Ok(Some(s)) => s,
                    Ok(None) => {
                        Status::error(&format!("Snapshot not found: {}", id));
                        return exit_codes::FAILURE;
                    }
                    Err(e) => {
                        Status::error(&format!("Failed to get snapshot: {}", e));
                        return exit_codes::FAILURE;
                    }
                }
            } else {
                Status::error("Please specify --latest or --snapshot <ID>");
                return exit_codes::FAILURE;
            };

            println!();
            println!("{}", format!("Restoring from snapshot: {}", snap.id).bold());
            println!("  Created: {}", snap.timestamp);
            println!("  Trigger: {}", snap.trigger);
            println!("  Files: {}", snap.files.len());
            println!();

            let files_to_restore = file.map(|f| vec![f]);
            match manager.restore_snapshot(&snap, files_to_restore.as_deref(), dry_run) {
                Ok(result) => {
                    print_restore_result(&result);

                    if result.failed_files.is_empty() {
                        exit_codes::SUCCESS
                    } else {
                        exit_codes::FAILURE
                    }
                }
                Err(e) => {
                    Status::error(&format!("Failed to restore: {}", e));
                    exit_codes::FAILURE
                }
            }
        }

        ProtectAction::CommitGuard => {
            let guard = match CommitGuard::new() {
                Ok(g) => g,
                Err(e) => {
                    Status::error(&format!("Failed to initialize commit guard: {}", e));
                    return exit_codes::FAILURE;
                }
            };

            match guard.show_pending_commit() {
                Ok(pending) => {
                    if pending.files.is_empty() {
                        Status::info("No staged changes to commit");
                    } else {
                        print_pending_commit(&pending);
                    }
                    exit_codes::SUCCESS
                }
                Err(e) => {
                    Status::error(&format!("Failed to analyze pending commit: {}", e));
                    exit_codes::FAILURE
                }
            }
        }

        ProtectAction::PushGuard { remote, branch } => {
            let guard = match PushGuard::new() {
                Ok(g) => g,
                Err(e) => {
                    Status::error(&format!("Failed to initialize push guard: {}", e));
                    return exit_codes::FAILURE;
                }
            };

            let branch_name = branch.unwrap_or_else(|| {
                foodshare_core::git::GitRepo::open_current()
                    .and_then(|r| r.current_branch())
                    .unwrap_or_else(|_| "main".to_string())
            });

            match guard.show_pending_push(&remote, &branch_name) {
                Ok(pending) => {
                    if pending.commits.is_empty() {
                        Status::info("Nothing to push - up to date with remote");
                    } else {
                        print_pending_push(&pending);
                    }
                    exit_codes::SUCCESS
                }
                Err(e) => {
                    Status::error(&format!("Failed to analyze pending push: {}", e));
                    exit_codes::FAILURE
                }
            }
        }

        ProtectAction::VerifyBuild { quick } => {
            match verify_build(quick) {
                Ok(result) => {
                    if result.success {
                        exit_codes::SUCCESS
                    } else {
                        println!();
                        println!("{}", "Build errors:".red().bold());
                        for error in result.errors.iter().take(10) {
                            println!("  {}", error);
                        }
                        exit_codes::FAILURE
                    }
                }
                Err(e) => {
                    Status::error(&format!("Build verification failed: {}", e));
                    exit_codes::FAILURE
                }
            }
        }

        ProtectAction::History { limit } => {
            let data_dir = std::path::Path::new(".foodshare-hooks");
            let history = match OperationHistory::new(data_dir) {
                Ok(h) => h,
                Err(e) => {
                    Status::error(&format!("Failed to load history: {}", e));
                    return exit_codes::FAILURE;
                }
            };

            match history.recent(limit) {
                Ok(records) => {
                    println!();
                    println!("{}", "═".repeat(70));
                    println!("{}", "OPERATION HISTORY".bold());
                    println!("{}", "═".repeat(70));
                    println!();

                    if records.is_empty() {
                        println!("  No operations recorded yet.");
                    } else {
                        for record in &records {
                            let status = if record.success {
                                "✓".green().to_string()
                            } else {
                                "✗".red().to_string()
                            };
                            println!(
                                "  {} {} {} - {} ({} files)",
                                status,
                                record.timestamp.format("%Y-%m-%d %H:%M"),
                                record.operation.to_string().cyan(),
                                record.details,
                                record.affected_files.len()
                            );
                        }
                    }
                    println!();
                    exit_codes::SUCCESS
                }
                Err(e) => {
                    Status::error(&format!("Failed to load history: {}", e));
                    exit_codes::FAILURE
                }
            }
        }

        ProtectAction::Status => {
            println!();
            println!("{}", "═".repeat(60));
            println!("{}", "CODE PROTECTION STATUS".bold());
            println!("{}", "═".repeat(60));
            println!();

            let config = ProtectionConfig::default();
            println!("  Configuration:");
            let snap_status = if config.snapshots_enabled { "✓".green().to_string() } else { "✗".red().to_string() };
            let build_status = if config.verify_build { "✓".green().to_string() } else { "✗".red().to_string() };
            let interactive_status = if config.interactive_approval { "✓".green().to_string() } else { "○".dimmed().to_string() };
            println!("    Snapshots enabled: {}", snap_status);
            println!("    Build verification: {}", build_status);
            println!("    Interactive approval: {}", interactive_status);
            println!("    Max snapshots: {}", config.max_snapshots);
            println!();

            println!("  Protected paths:");
            for path in &config.protected_paths {
                println!("    {} {}", "•".dimmed(), path);
            }
            println!();

            // Count snapshots
            if let Ok(manager) = SnapshotManager::new(config.clone()) {
                if let Ok(snapshots) = manager.list_snapshots() {
                    println!("  Snapshots: {} stored", snapshots.len());
                    if let Some(latest) = snapshots.first() {
                        println!("  Latest: {} ({})", latest.id, latest.timestamp.format("%Y-%m-%d %H:%M"));
                    }
                }
            }

            println!();
            println!("{}", "═".repeat(60));
            exit_codes::SUCCESS
        }
    }
}
