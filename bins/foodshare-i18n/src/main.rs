//! Foodshare i18n CLI - Enterprise Translation Management
//!
//! A comprehensive CLI tool for managing translations across the Foodshare platform.

use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::process::ExitCode;

mod api;
mod commands;
mod config;
mod types;

use commands::{audit, backfill, deploy, generate_infoplist, health, test, translate, update};

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
    /// Check system health and status
    Health {
        /// Include response times and detailed metrics
        #[arg(short, long)]
        detailed: bool,
    },

    /// Test translation system
    Test {
        #[command(subcommand)]
        target: TestTarget,
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
        /// Target locale (or "all" to sync all locales)
        locale: String,

        /// Apply translations (dry-run if not specified)
        #[arg(short, long)]
        apply: bool,

        /// Maximum keys to translate per locale
        #[arg(short, long, default_value = "50")]
        limit: usize,
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

    /// Deploy translation system
    Deploy {
        /// Skip database migrations
        #[arg(long)]
        no_migrations: bool,

        /// Skip deploying edge functions
        #[arg(long)]
        no_functions: bool,

        /// Skip endpoint testing after deployment
        #[arg(long)]
        no_test: bool,
    },

    /// Update translations
    Update {
        /// Target locale
        locale: String,

        /// Path to JSON file with translations (if not specified, uses preset translations)
        #[arg(short, long)]
        file: Option<String>,
    },

    /// Backfill translations for existing posts
    Backfill {
        /// Number of posts to process per batch
        #[arg(short, long, default_value = "10")]
        batch_size: usize,

        /// Delay between batches in milliseconds
        #[arg(short, long, default_value = "2000")]
        delay: u64,

        /// Maximum number of posts to process
        #[arg(short, long)]
        limit: Option<usize>,

        /// Dry run (don't actually send translation requests)
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate localized InfoPlist.strings files for iOS
    GenerateInfoplist {
        /// Dry run (preview translations without writing files)
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum TestTarget {
    /// Test translation fetch for a locale
    Fetch {
        /// Locale to test
        #[arg(default_value = "en")]
        locale: String,

        /// Test delta sync
        #[arg(short, long)]
        delta: bool,

        /// Test ETag caching
        #[arg(short, long)]
        cache: bool,
    },

    /// Test LLM translation endpoint
    Llm {
        /// Text to translate
        #[arg(short, long, default_value = "Fresh apples from my garden, ready to share!")]
        text: String,

        /// Source language
        #[arg(short, long, default_value = "en")]
        source: String,

        /// Target language
        #[arg(long, default_value = "es")]
        target: String,

        /// Translation context
        #[arg(long, default_value = "food-sharing platform")]
        context: String,
    },

    /// Test post translation system end-to-end
    Posts {
        /// Target locale to test
        #[arg(short, long, default_value = "ru")]
        locale: String,

        /// Number of posts to test
        #[arg(short = 'n', long, default_value = "5")]
        limit: usize,

        /// Skip triggering new translations (only test existing)
        #[arg(long)]
        skip_trigger: bool,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("foodshare_i18n=debug")
            .init();
    }

    let result = match cli.command {
        Commands::Health { detailed } => health::run(detailed, &cli.format).await,

        Commands::Test { target } => match target {
            TestTarget::Fetch { locale, delta, cache } => {
                test::run(&locale, delta, cache, &cli.format).await
            }
            TestTarget::Llm { text, source, target, context } => {
                test::run_llm(&text, &source, &target, &context, &cli.format).await
            }
            TestTarget::Posts { locale, limit, skip_trigger } => {
                test::run_posts(&locale, limit, skip_trigger, cli.verbose, &cli.format).await
            }
        },

        Commands::Audit { locale, missing, limit } => {
            audit::run(locale.as_deref(), missing, limit, &cli.format).await
        }

        Commands::Translate { locale, apply, limit } => {
            if locale == "all" {
                translate::sync_all(apply, &cli.format).await
            } else {
                translate::run(&locale, apply, limit, &cli.format).await
            }
        }

        Commands::Bench { count, locale } => {
            commands::bench::run(count, &locale, &cli.format).await
        }

        Commands::Locales => commands::locales::run(&cli.format).await,

        Commands::Deploy { no_migrations, no_functions, no_test } => {
            deploy::run(!no_migrations, !no_functions, !no_test, &cli.format).await
        }

        Commands::Update { locale, file } => {
            if let Some(file_path) = file {
                update::run_from_file(&locale, &file_path, &cli.format).await
            } else {
                update::run_preset(&locale, &cli.format).await
            }
        }

        Commands::Backfill { batch_size, delay, limit, dry_run } => {
            backfill::run(batch_size, delay, limit, dry_run, &cli.format).await
        }

        Commands::GenerateInfoplist { dry_run } => {
            generate_infoplist::run(dry_run, &cli.format).await
        }
    };

    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            ExitCode::FAILURE
        }
    }
}
