//! Foodshare Android CLI
//!
//! Git hooks and development tools for Foodshare Android.

use anyhow::Result;
use clap::{Parser, Subcommand};
use foodshare_cli::output::Status;
use foodshare_core::config::Config;
use foodshare_core::error::exit_codes;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "foodshare-android")]
#[command(about = "Git hooks and development tools for Foodshare Android")]
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
    /// Format code (Kotlin + Swift)
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
        /// Language: kotlin, swift, both
        #[arg(long, default_value = "both")]
        lang: String,
    },

    /// Lint code (Kotlin + Swift)
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
        /// Language: kotlin, swift, both
        #[arg(long, default_value = "both")]
        lang: String,
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
        /// Build bundle (AAB) instead of APK
        #[arg(long)]
        bundle: bool,
    },

    /// Run tests
    Test {
        /// Enable coverage
        #[arg(long)]
        coverage: bool,
    },

    /// Manage emulators
    Emulator {
        /// Action: list, boot, shutdown
        action: String,
        /// AVD name
        #[arg(long)]
        name: Option<String>,
    },

    /// Build Swift for Android
    #[command(name = "swift-build")]
    SwiftBuild {
        /// Target: arm64, x86_64, all
        #[arg(long, default_value = "arm64")]
        target: String,
        /// Configuration: debug, release
        #[arg(long, default_value = "debug")]
        configuration: String,
    },

    /// Generate Swift-Java bindings
    #[command(name = "swift-java")]
    SwiftJava {
        /// Action: generate, verify
        action: String,
    },

    /// Build FoodshareCore Swift library for Android
    #[command(name = "swift-core")]
    SwiftCore {
        #[command(subcommand)]
        action: SwiftCoreAction,
    },

    /// Diagnose environment
    Doctor {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Verify setup
    Verify,
}

#[derive(Subcommand)]
enum SwiftCoreAction {
    /// Check prerequisites for building Swift for Android
    Check,
    /// Build FoodshareCore for Android
    Build {
        /// Target: arm64, x86_64, all
        #[arg(long, default_value = "all")]
        target: String,
        /// Configuration: debug, release
        #[arg(long, default_value = "debug")]
        configuration: String,
        /// FoodshareCore project directory
        #[arg(long, default_value = "../FoodshareCore")]
        project_dir: PathBuf,
        /// Output directory for built libraries
        #[arg(long, default_value = "android-libs")]
        output_dir: PathBuf,
        /// Android API level
        #[arg(long, default_value = "28")]
        api_level: u8,
    },
    /// Copy built libraries to Android project
    Copy {
        /// Source directory with built libraries
        #[arg(long, default_value = "android-libs")]
        source_dir: PathBuf,
        /// Android project directory
        #[arg(long, default_value = ".")]
        android_dir: PathBuf,
    },
    /// Print setup instructions
    Setup,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.no_color {
        owo_colors::set_override(false);
    }

    let config = Config::load(cli.config.as_deref().map(|p| p.to_str().unwrap()))?;

    let exit_code = match cli.command {
        Commands::Format { files, check, staged, lang } => {
            run_format(&files, check, staged, &lang)
        }
        Commands::Lint { files, strict, fix, lang } => {
            run_lint(&files, strict, fix, &lang)
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
        Commands::Build { configuration, clean, bundle } => {
            run_build(&configuration, clean, bundle)
        }
        Commands::Test { coverage } => {
            run_test(coverage)
        }
        Commands::Emulator { action, name } => {
            run_emulator(&action, name.as_deref())
        }
        Commands::SwiftBuild { target, configuration } => {
            run_swift_build(&target, &configuration)
        }
        Commands::SwiftJava { action } => {
            run_swift_java(&action)
        }
        Commands::SwiftCore { action } => {
            run_swift_core(action)
        }
        Commands::Doctor { json } => {
            run_doctor(json)
        }
        Commands::Verify => {
            run_verify()
        }
    };

    std::process::exit(exit_code);
}

fn run_format(_files: &[PathBuf], _check: bool, _staged: bool, lang: &str) -> i32 {
    use foodshare_android::kotlin_tools;

    if lang == "kotlin" || lang == "both" {
        if !kotlin_tools::has_ktlint() {
            Status::error("ktlint not found. Install with: brew install ktlint");
            return exit_codes::FAILURE;
        }

        Status::info("Formatting Kotlin files...");
        match kotlin_tools::format_directory(std::path::Path::new("app")) {
            Ok(result) => {
                if result.success {
                    Status::success("Kotlin formatting complete");
                } else {
                    Status::error("Kotlin formatting failed");
                    return exit_codes::FAILURE;
                }
            }
            Err(e) => {
                Status::error(&format!("Format error: {}", e));
                return exit_codes::FAILURE;
            }
        }
    }

    if lang == "swift" || lang == "both" {
        Status::info("Swift formatting for Android not yet implemented");
    }

    exit_codes::SUCCESS
}

fn run_lint(_files: &[PathBuf], strict: bool, _fix: bool, lang: &str) -> i32 {
    use foodshare_android::kotlin_tools;

    if lang == "kotlin" || lang == "both" {
        if !kotlin_tools::has_ktlint() {
            Status::error("ktlint not found");
            return exit_codes::FAILURE;
        }

        Status::info("Linting Kotlin files...");
        match kotlin_tools::check_directory(std::path::Path::new("app")) {
            Ok(result) => {
                if result.success {
                    Status::success("Kotlin lint passed");
                } else {
                    Status::error("Kotlin lint found issues");
                    println!("{}", result.stdout);
                    if strict {
                        return exit_codes::FAILURE;
                    }
                }
            }
            Err(e) => {
                Status::error(&format!("Lint error: {}", e));
                return exit_codes::FAILURE;
            }
        }
    }

    exit_codes::SUCCESS
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
        foodshare_core::file_scanner::scan_kotlin_files(std::path::Path::new("."))
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

fn run_migrations(dir: &std::path::Path) -> i32 {
    use foodshare_hooks::migrations;

    match migrations::check_migrations(dir, true, true) {
        Ok(check) => migrations::print_results(&check),
        Err(e) => {
            Status::error(&format!("Migration check error: {}", e));
            exit_codes::FAILURE
        }
    }
}

fn run_build(configuration: &str, clean: bool, bundle: bool) -> i32 {
    use foodshare_android::gradle;

    let project_dir = std::path::Path::new(".");

    if clean {
        Status::info("Cleaning...");
        if let Err(e) = gradle::clean(project_dir) {
            Status::error(&format!("Clean failed: {}", e));
            return exit_codes::FAILURE;
        }
    }

    Status::info(&format!("Building {} {}...", 
        configuration,
        if bundle { "bundle" } else { "APK" }
    ));

    let result = if bundle {
        if configuration == "release" {
            gradle::bundle_release(project_dir)
        } else {
            gradle::bundle_debug(project_dir)
        }
    } else if configuration == "release" {
        gradle::build_release(project_dir)
    } else {
        gradle::build_debug(project_dir)
    };

    match result {
        Ok(r) => {
            if r.success {
                Status::success("Build succeeded");
                exit_codes::SUCCESS
            } else {
                Status::error("Build failed");
                eprintln!("{}", r.stderr);
                exit_codes::FAILURE
            }
        }
        Err(e) => {
            Status::error(&format!("Build error: {}", e));
            exit_codes::FAILURE
        }
    }
}

fn run_test(_coverage: bool) -> i32 {
    use foodshare_android::gradle;

    Status::info("Running tests...");

    match gradle::test(std::path::Path::new(".")) {
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

fn run_emulator(action: &str, name: Option<&str>) -> i32 {
    use foodshare_android::emulator;

    match action {
        "list" => {
            match emulator::list_avds() {
                Ok(avds) => {
                    println!("Available AVDs:");
                    for avd in avds {
                        println!("  - {}", avd);
                    }
                    exit_codes::SUCCESS
                }
                Err(e) => {
                    Status::error(&format!("Failed to list AVDs: {}", e));
                    exit_codes::FAILURE
                }
            }
        }
        "boot" => {
            let avd_name = name.unwrap_or("Pixel_7_API_34");
            Status::info(&format!("Booting {}...", avd_name));
            match emulator::boot(avd_name) {
                Ok(_) => {
                    Status::success(&format!("Started {}", avd_name));
                    exit_codes::SUCCESS
                }
                Err(e) => {
                    Status::error(&format!("Failed to boot: {}", e));
                    exit_codes::FAILURE
                }
            }
        }
        "shutdown" => {
            match emulator::shutdown_all() {
                Ok(_) => {
                    Status::success("Shutdown all emulators");
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

fn run_swift_build(target: &str, configuration: &str) -> i32 {
    use foodshare_android::swift_android::{self, AndroidTarget};

    if !swift_android::has_swift() {
        Status::error("Swift not found");
        return exit_codes::FAILURE;
    }

    let android_target = match target {
        "arm64" => AndroidTarget::Arm64,
        "x86_64" => AndroidTarget::X86_64,
        _ => {
            Status::error(&format!("Unknown target: {}", target));
            return exit_codes::FAILURE;
        }
    };

    Status::info(&format!("Building Swift for {} ({})...", target, configuration));

    match swift_android::build(
        std::path::Path::new("swift-core"),
        android_target,
        configuration,
        28,
    ) {
        Ok(result) => {
            if result.success {
                Status::success("Swift build succeeded");
                exit_codes::SUCCESS
            } else {
                Status::error("Swift build failed");
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

fn run_swift_java(action: &str) -> i32 {
    use foodshare_android::swift_android;

    match action {
        "verify" => {
            if swift_android::has_swift_java() {
                Status::success("swift-java is installed");
                exit_codes::SUCCESS
            } else {
                Status::error("swift-java not found");
                exit_codes::FAILURE
            }
        }
        "generate" => {
            Status::info("Generating bindings...");
            match swift_android::generate_bindings(
                std::path::Path::new("swift-core/Sources"),
                std::path::Path::new("app/src/main/java"),
                "com.foodshare.swift",
            ) {
                Ok(result) => {
                    if result.success {
                        Status::success("Bindings generated");
                        exit_codes::SUCCESS
                    } else {
                        Status::error("Binding generation failed");
                        eprintln!("{}", result.stderr);
                        exit_codes::FAILURE
                    }
                }
                Err(e) => {
                    Status::error(&format!("Generation error: {}", e));
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

fn run_doctor(_json: bool) -> i32 {
    use foodshare_android::{emulator, kotlin_tools, swift_android};

    println!("Environment Check");
    println!();

    // Kotlin tools
    if kotlin_tools::has_ktlint() {
        Status::success("ktlint: installed");
    } else {
        Status::warning("ktlint: not found");
    }

    if kotlin_tools::has_detekt() {
        Status::success("detekt: installed");
    } else {
        Status::warning("detekt: not found");
    }

    // Android tools
    if emulator::is_adb_available() {
        Status::success("adb: installed");
    } else {
        Status::error("adb: not found");
    }

    if emulator::is_emulator_available() {
        Status::success("emulator: installed");
    } else {
        Status::warning("emulator: not found");
    }

    // Swift tools
    if swift_android::has_swift() {
        if let Ok(version) = swift_android::swift_version() {
            Status::success(&format!("Swift: {}", version));
        }
    } else {
        Status::warning("Swift: not found");
    }

    if swift_android::has_swift_java() {
        Status::success("swift-java: installed");
    } else {
        Status::warning("swift-java: not found");
    }

    exit_codes::SUCCESS
}

fn run_verify() -> i32 {
    Status::info("Verifying setup...");

    if foodshare_core::process::command_exists("lefthook") {
        Status::success("lefthook: installed");
    } else {
        Status::error("lefthook: not found");
        return exit_codes::FAILURE;
    }

    Status::success("Setup verified");
    exit_codes::SUCCESS
}

fn run_swift_core(action: SwiftCoreAction) -> i32 {
    use foodshare_android::swift_core::{
        self, BuildConfig, SwiftAndroidTarget,
    };
    use owo_colors::OwoColorize;

    match action {
        SwiftCoreAction::Check => {
            match swift_core::check_prerequisites() {
                Ok(status) => {
                    status.print_status();
                    if status.is_ready() {
                        println!();
                        Status::success("Ready to build Swift for Android");
                        exit_codes::SUCCESS
                    } else {
                        println!();
                        Status::error("Prerequisites not met");
                        swift_core::print_setup_instructions();
                        exit_codes::FAILURE
                    }
                }
                Err(e) => {
                    Status::error(&format!("Check failed: {}", e));
                    exit_codes::FAILURE
                }
            }
        }
        SwiftCoreAction::Build {
            target,
            configuration,
            project_dir,
            output_dir,
            api_level,
        } => {
            println!("{}", "Building FoodshareCore for Android".bold());
            println!();

            // Check prerequisites first
            match swift_core::check_prerequisites() {
                Ok(status) => {
                    if !status.is_ready() {
                        status.print_status();
                        Status::error("Prerequisites not met. Run 'swift-core setup' for instructions.");
                        return exit_codes::FAILURE;
                    }
                }
                Err(e) => {
                    Status::error(&format!("Prerequisite check failed: {}", e));
                    return exit_codes::FAILURE;
                }
            }

            let config = BuildConfig {
                project_dir,
                output_dir,
                api_level,
                configuration,
                static_stdlib: true,
            };

            let results = match target.as_str() {
                "arm64" => {
                    swift_core::build_for_target(SwiftAndroidTarget::Arm64, &config)
                        .map(|r| vec![r])
                }
                "x86_64" => {
                    swift_core::build_for_target(SwiftAndroidTarget::X86_64, &config)
                        .map(|r| vec![r])
                }
                "all" => swift_core::build_all(&config),
                _ => {
                    Status::error(&format!("Unknown target: {}. Use arm64, x86_64, or all", target));
                    return exit_codes::FAILURE;
                }
            };

            match results {
                Ok(build_results) => {
                    println!();
                    let success_count = build_results.iter().filter(|r| r.success).count();
                    let total = build_results.len();

                    if success_count == total {
                        Status::success(&format!("Built {} target(s) successfully", total));
                        exit_codes::SUCCESS
                    } else {
                        for result in &build_results {
                            if !result.success {
                                Status::error(&format!(
                                    "{}: {}",
                                    result.target.display_name(),
                                    result.error.as_deref().unwrap_or("Unknown error")
                                ));
                            }
                        }
                        exit_codes::FAILURE
                    }
                }
                Err(e) => {
                    Status::error(&format!("Build failed: {}", e));
                    exit_codes::FAILURE
                }
            }
        }
        SwiftCoreAction::Copy { source_dir, android_dir } => {
            Status::info("Copying libraries to Android project...");

            match swift_core::copy_to_android_project(&source_dir, &android_dir) {
                Ok(()) => {
                    Status::success("Libraries copied successfully");
                    exit_codes::SUCCESS
                }
                Err(e) => {
                    Status::error(&format!("Copy failed: {}", e));
                    exit_codes::FAILURE
                }
            }
        }
        SwiftCoreAction::Setup => {
            swift_core::print_setup_instructions();
            exit_codes::SUCCESS
        }
    }
}
