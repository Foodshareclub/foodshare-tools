//! Foodshare i18n CLI - Enterprise Translation Management
//!
//! A comprehensive CLI tool for managing translations across the Foodshare platform.
//!
//! # Features
//! - Health checks for all translation endpoints
//! - Translation coverage auditing
//! - Delta sync testing
//! - Performance benchmarking
//! - Auto-translation with AI

use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::process::ExitCode;

mod api;
mod commands;
mod config;
mod types;

use commands::{audit, health, status, test, translate};

/// Enterprise Translation Management CLI for Foodshare
#[derive(Parser)]
#[command(name = "foodshare-i18n")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Output format (text, json)
    #[arg(short, long, global = true, default_value = "text")]
    format: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show overall translation system status
    Status,

    /// Check health of all translation endpoints
    Health {
        /// Include response times
        #[arg(short, long)]
        timing: bool,
    },

    /// Test translation fetch for a locale
    Test {
        /// Locale to test (default: en)
        #[arg(default_value = "en")]
        locale: String,

        /// Test delta sync
        #[arg(short, long)]
        delta: bool,

        /// Test ETag caching
        #[arg(short, long)]
        cache: bool,
    },

    /// Audit translation coverage
    Audit {
        /// Specific locale to audit (audits all if not specified)
        locale: Option<String>,

        /// Show missing keys
        #[arg(short, long)]
        missing: bool,

        /// Limit number of missing keys to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Auto-translate missing keys
    Translate {
        /// Target locale
        locale: String,

        /// Apply translations (dry-run if not specified)
        #[arg(short, long)]
        apply: bool,

        /// Maximum keys to translate
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },

    /// Sync all locales
    Sync {
        /// Apply translations
        #[arg(short, long)]
        apply: bool,
    },

    /// Benchmark translation endpoints
    Bench {
        /// Number of requests
        #[arg(short, long, default_value = "10")]
        count: usize,

        /// Locale to benchmark
        #[arg(short, long, default_value = "en")]
        locale: String,
    },

    /// List supported locales
    Locales,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("foodshare_i18n=debug")
            .init();
    }

    let result = match cli.command {
        Commands::Status => status::run(&cli.format).await,
        Commands::Health { timing } => health::run(timing, &cli.format).await,
        Commands::Test { locale, delta, cache } => {
            test::run(&locale, delta, cache, &cli.format).await
        }
        Commands::Audit { locale, missing, limit } => {
            audit::run(locale.as_deref(), missing, limit, &cli.format).await
        }
        Commands::Translate { locale, apply, limit } => {
            translate::run(&locale, apply, limit, &cli.format).await
        }
        Commands::Sync { apply } => translate::sync_all(apply, &cli.format).await,
        Commands::Bench { count, locale } => commands::bench::run(count, &locale, &cli.format).await,
        Commands::Locales => commands::locales::run(&cli.format).await,
    };

    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            ExitCode::FAILURE
        }
    }
}
