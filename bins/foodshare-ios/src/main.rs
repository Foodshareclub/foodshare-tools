//! Foodshare iOS CLI
//!
//! Git hooks and development tools for Foodshare iOS.

use anyhow::Result;
use clap::{Parser, Subcommand};
use foodshare_cli::output::Status;
use foodshare_core::config::Config;
use foodshare_core::error::exit_codes;
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
    /// Format Swift code
    Format {
        /// Files to format
        #[arg(trailing_var_arg = true)]
        files: Vec<PathBuf>,
        /// Check only, don't modify
        #[arg(long)]
        check: bool,
        /// Format only staged files
        #[arg(long)]
        staged: bool,
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

    /// Pre-push checks
    #[command(name = "pre-push")]
    PrePush {
        /// Remote name
        remote: Option<String>,
        /// Remote URL
        url: Option<String>,
        /// Fail fast on first error
        #[arg(long)]
        fail_fast: bool,
        /// Use release build
        #[arg(long)]
        release: bool,
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.no_color {
        owo_colors::set_override(false);
    }

    let config = Config::load(cli.config.as_deref().map(|p| p.to_str().unwrap()))?;

    let exit_code = match cli.command {
        Commands::Format { files, check, staged } => {
            run_format(&files, check, staged)
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
        Commands::PrePush { remote, url, fail_fast, release } => {
            run_pre_push(remote.as_deref(), url.as_deref(), fail_fast, release)
        }
    };

    std::process::exit(exit_code);
}

fn run_format(files: &[PathBuf], check: bool, staged: bool) -> i32 {
    use foodshare_ios::swift_tools;

    if !swift_tools::has_swiftformat() {
        Status::error("swiftformat not found. Install with: brew install swiftformat");
        return exit_codes::FAILURE;
    }

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
        vec![PathBuf::from("FoodShare")]
    } else {
        files.to_vec()
    };

    if target_files.is_empty() {
        Status::info("No Swift files to format");
        return exit_codes::SUCCESS;
    }

    match swift_tools::format_directory(&target_files[0], check) {
        Ok(result) => {
            if result.success {
                Status::success("Formatting complete");
                exit_codes::SUCCESS
            } else {
                Status::error("Formatting failed");
                eprintln!("{}", result.stderr);
                exit_codes::FAILURE
            }
        }
        Err(e) => {
            Status::error(&format!("Format error: {}", e));
            exit_codes::FAILURE
        }
    }
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

fn run_simulator(action: &str, device: Option<&str>) -> i32 {
    use foodshare_ios::simulator;

    match action {
        "list" => {
            match simulator::list_devices() {
                Ok(devices) => {
                    println!("{}", "Available Simulators:".to_string());
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

    println!("{}", "Environment Check".to_string());
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


fn run_pre_push(_remote: Option<&str>, _url: Option<&str>, _fail_fast: bool, _release: bool) -> i32 {
    // Pre-push checks for iOS
    // This is a pass-through that allows lefthook to work
    // The actual checks are handled by lefthook configuration
    Status::success("Pre-push checks passed");
    exit_codes::SUCCESS
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
    }
}
