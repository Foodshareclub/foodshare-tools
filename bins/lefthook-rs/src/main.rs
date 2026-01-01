//! Lefthook-rs - Fast git hooks for Foodshare web
//!
//! OWASP security scanning and development tools for Next.js/React.

use anyhow::Result;
use clap::{Parser, Subcommand};
use foodshare_cli::output::Status;
use foodshare_core::config::Config;
use foodshare_core::error::exit_codes;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "lefthook-rs")]
#[command(about = "Fast git hooks for Foodshare web")]
#[command(version)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Security checks (secrets, credentials, debug statements)
    Security {
        /// Files to check
        #[arg(trailing_var_arg = true)]
        files: Vec<String>,
    },

    /// Validate conventional commit message format
    ConventionalCommit {
        /// Path to commit message file
        #[arg(required = true)]
        message_file: String,
    },

    /// Check for protected branch push
    ProtectedBranch,

    /// Check for large files in staging
    LargeFiles {
        /// Maximum file size in KB
        #[arg(short, long, default_value = "500")]
        max_size: u64,
    },

    /// Next.js/React/Vercel security vulnerabilities check
    NextjsSecurity {
        /// Files to check
        #[arg(trailing_var_arg = true)]
        files: Vec<String>,
    },

    /// Check accessibility in JSX/TSX files
    Accessibility {
        /// Files to check
        #[arg(trailing_var_arg = true)]
        files: Vec<String>,
    },

    /// Analyze bundle size
    BundleSize {
        /// Threshold in KB
        #[arg(long)]
        threshold: Option<u64>,
    },

    /// Run all pre-commit checks
    PreCommit {
        /// Files to check
        #[arg(trailing_var_arg = true)]
        files: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::default();

    let result = match cli.command {
        Commands::Security { files } => run_security(&files, &config),
        Commands::ConventionalCommit { message_file } => run_conventional_commit(&message_file, &config),
        Commands::ProtectedBranch => run_protected_branch(),
        Commands::LargeFiles { max_size } => run_large_files(max_size),
        Commands::NextjsSecurity { files } => run_nextjs_security(&files),
        Commands::Accessibility { files } => run_accessibility(&files),
        Commands::BundleSize { threshold } => run_bundle_size(threshold),
        Commands::PreCommit { files } => run_pre_commit(&files, &config),
    };

    std::process::exit(result);
}

fn run_security(files: &[String], config: &Config) -> i32 {
    use foodshare_hooks::secrets;

    let paths: Vec<PathBuf> = if files.is_empty() {
        foodshare_core::git::GitRepo::open_current()
            .and_then(|r| r.staged_files())
            .unwrap_or_default()
    } else {
        files.iter().map(PathBuf::from).collect()
    };

    match secrets::scan_files(&paths, &config.schema.secrets) {
        Ok(matches) => secrets::print_results(&matches),
        Err(e) => {
            Status::error(&format!("Scan error: {}", e));
            exit_codes::FAILURE
        }
    }
}

fn run_conventional_commit(message_file: &str, config: &Config) -> i32 {
    use foodshare_hooks::commit_msg;

    let path = PathBuf::from(message_file);

    match commit_msg::validate_commit_message(&path, &config.schema.commit_msg) {
        Ok(result) => {
            if result.valid {
                Status::success(result.message.as_deref().unwrap_or("Valid"));
            } else {
                commit_msg::print_error(
                    &std::fs::read_to_string(&path).unwrap_or_default(),
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

fn run_protected_branch() -> i32 {
    use foodshare_core::git::GitRepo;

    let protected = ["main", "master", "production", "staging"];

    match GitRepo::open_current().and_then(|r| r.current_branch()) {
        Ok(branch) => {
            if protected.contains(&branch.as_str()) {
                Status::warning(&format!(
                    "You are on protected branch '{}'. Are you sure you want to push?",
                    branch
                ));
                // Return success but with warning - actual blocking should be in hook config
                exit_codes::SUCCESS
            } else {
                exit_codes::SUCCESS
            }
        }
        Err(e) => {
            Status::error(&format!("Git error: {}", e));
            exit_codes::FAILURE
        }
    }
}

fn run_large_files(max_size_kb: u64) -> i32 {
    use foodshare_core::git::GitRepo;

    let max_bytes = max_size_kb * 1024;

    match GitRepo::open_current().and_then(|r| r.staged_files()) {
        Ok(files) => {
            let mut large_files = Vec::new();

            for file in files {
                if let Ok(metadata) = std::fs::metadata(&file) {
                    if metadata.len() > max_bytes {
                        large_files.push((file, metadata.len()));
                    }
                }
            }

            if large_files.is_empty() {
                Status::success("No large files detected");
                exit_codes::SUCCESS
            } else {
                Status::error(&format!("Found {} large file(s):", large_files.len()));
                for (file, size) in large_files {
                    eprintln!(
                        "  - {} ({:.2} KB)",
                        file.display(),
                        size as f64 / 1024.0
                    );
                }
                exit_codes::FAILURE
            }
        }
        Err(e) => {
            Status::error(&format!("Git error: {}", e));
            exit_codes::FAILURE
        }
    }
}

fn run_nextjs_security(files: &[String]) -> i32 {
    use foodshare_web::nextjs_security;

    let paths: Vec<PathBuf> = if files.is_empty() {
        foodshare_core::file_scanner::scan_ts_files(std::path::Path::new("src"))
            .unwrap_or_default()
    } else {
        files.iter().map(PathBuf::from).collect()
    };

    if paths.is_empty() {
        Status::info("No files to scan");
        return exit_codes::SUCCESS;
    }

    match nextjs_security::scan_files(&paths) {
        Ok(findings) => nextjs_security::print_results(&findings),
        Err(e) => {
            Status::error(&format!("Scan error: {}", e));
            exit_codes::FAILURE
        }
    }
}

fn run_accessibility(files: &[String]) -> i32 {
    use foodshare_web::accessibility;

    let paths: Vec<PathBuf> = if files.is_empty() {
        foodshare_core::file_scanner::FileScanner::new("src")
            .with_extensions(&["jsx", "tsx"])
            .scan()
            .unwrap_or_default()
    } else {
        files.iter().map(PathBuf::from).collect()
    };

    if paths.is_empty() {
        Status::info("No JSX/TSX files to check");
        return exit_codes::SUCCESS;
    }

    match accessibility::check_files(&paths) {
        Ok(issues) => accessibility::print_results(&issues),
        Err(e) => {
            Status::error(&format!("Check error: {}", e));
            exit_codes::FAILURE
        }
    }
}

fn run_bundle_size(threshold: Option<u64>) -> i32 {
    use foodshare_web::bundle_size;

    let build_dir = std::path::Path::new(".");

    match bundle_size::analyze_nextjs_build(build_dir) {
        Ok(analysis) => {
            bundle_size::print_analysis(&analysis, threshold);
            exit_codes::SUCCESS
        }
        Err(e) => {
            Status::error(&format!("Analysis error: {}", e));
            exit_codes::FAILURE
        }
    }
}

fn run_pre_commit(files: &[String], config: &Config) -> i32 {
    Status::info("Running pre-commit checks...");

    // Security check
    let security_result = run_security(files, config);
    if security_result != exit_codes::SUCCESS {
        return security_result;
    }

    // Large files check
    let large_files_result = run_large_files(500);
    if large_files_result != exit_codes::SUCCESS {
        return large_files_result;
    }

    Status::success("All pre-commit checks passed");
    exit_codes::SUCCESS
}
