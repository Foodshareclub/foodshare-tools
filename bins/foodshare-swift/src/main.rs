//! Foodshare Swift Toolchain Manager
//!
//! CLI tool for managing Swift versions across the Foodshare monorepo.

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use owo_colors::OwoColorize;
use foodshare_swift_toolchain::{
    detect::SwiftToolchain, migrate::SwiftMigrator, verify::VerificationReport,
    REQUIRED_SWIFT_VERSION,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "foodshare-swift")]
#[command(about = "Swift toolchain version management for Foodshare")]
#[command(version)]
#[command(author)]
struct Cli {
    /// Project root directory
    #[arg(short, long, default_value = ".")]
    project_root: PathBuf,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    format: String,

    /// Increase output verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Detect installed Swift version and toolchains
    Detect {
        /// List all available toolchains
        #[arg(long)]
        all: bool,
    },

    /// Verify Swift version consistency across the project
    Verify {
        /// Required Swift version (default: 6.3)
        #[arg(short, long)]
        required: Option<String>,
    },

    /// Configure environment for Swift version
    Configure {
        /// Swift version to configure
        version: String,

        /// Generate shell export commands
        #[arg(long)]
        export: bool,
    },

    /// Migrate project to new Swift version
    Migrate {
        /// Source Swift version
        #[arg(short, long)]
        from: String,

        /// Target Swift version
        #[arg(short, long)]
        to: String,

        /// Dry run - don't modify files
        #[arg(long)]
        dry_run: bool,
    },

    /// Use specific Swift version (configure environment)
    Use {
        /// Swift version or toolchain name
        version: String,
    },

    /// List available Swift toolchains
    List,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Detect { all } => cmd_detect(all, &cli.format)?,
        Commands::Verify { required } => {
            cmd_verify(&cli.project_root, required.as_deref(), &cli.format)?
        }
        Commands::Configure { version, export } => cmd_configure(&version, export)?,
        Commands::Migrate { from, to, dry_run } => {
            cmd_migrate(&cli.project_root, &from, &to, dry_run)?
        }
        Commands::Use { version } => cmd_use(&version)?,
        Commands::List => cmd_list()?,
    }

    Ok(())
}

fn cmd_detect(all: bool, format: &str) -> Result<()> {
    let active = SwiftToolchain::detect_active()?;

    if format == "json" {
        let json = serde_json::json!({
            "active": {
                "version": active.version.raw,
                "path": active.path,
                "is_xcode": active.is_xcode,
            }
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("\n{}", "🔍 Swift Toolchain Detection".bold());
        println!("{}", "============================".bold());
        println!();
        println!("📦 Active Swift: {}", active.version.raw.cyan());
        println!("📍 Path: {}", active.path.display().dimmed());
        println!(
            "🔧 Source: {}",
            if active.is_xcode {
                "Xcode".yellow()
            } else {
                "Standalone".green()
            }
        );
        println!();
    }

    if all {
        let toolchains = SwiftToolchain::list_available()?;
        if !toolchains.is_empty() {
            println!("{}", "Available Toolchains:".bold());
            for tc in toolchains {
                println!(
                    "  • {} ({})",
                    tc.version.short_version().cyan(),
                    tc.path.parent().unwrap().display().dimmed()
                );
            }
            println!();
        }
    }

    Ok(())
}

fn cmd_verify(project_root: &PathBuf, required: Option<&str>, format: &str) -> Result<()> {
    let required_version = required.unwrap_or(REQUIRED_SWIFT_VERSION);
    let report = VerificationReport::generate(project_root, required_version)?;

    if format == "json" {
        println!("{}", report.to_json()?);
    } else {
        report.print();
    }

    if !report.all_match {
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_configure(version: &str, export: bool) -> Result<()> {
    let toolchains = SwiftToolchain::list_available()?;
    let matching = toolchains
        .iter()
        .find(|tc| tc.version.matches(version))
        .ok_or_else(|| anyhow::anyhow!("No toolchain found for Swift {}", version))?;

    if export {
        // Generate shell export commands
        println!("export TOOLCHAINS=swift");
        println!(
            "export PATH=\"{}/usr/bin:$PATH\"",
            matching
                .path
                .parent()
                .and_then(|p| p.parent())
                .unwrap()
                .display()
        );
    } else {
        println!("\n{}", "🔧 Swift Configuration".bold());
        println!("{}", "=====================".bold());
        println!();
        println!("Version: {}", matching.version.raw.cyan());
        println!(
            "Toolchain: {}",
            matching
                .path
                .parent()
                .and_then(|p| p.parent())
                .unwrap()
                .display()
                .dimmed()
        );
        println!();
        println!("{}", "To use this toolchain:".bold());
        println!();
        println!("  export TOOLCHAINS=swift");
        println!(
            "  export PATH=\"{}/usr/bin:$PATH\"",
            matching
                .path
                .parent()
                .and_then(|p| p.parent())
                .unwrap()
                .display()
        );
        println!();
        println!("Or add to your shell profile (~/.zshrc or ~/.bash_profile)");
        println!();
    }

    Ok(())
}

fn cmd_migrate(project_root: &PathBuf, from: &str, to: &str, dry_run: bool) -> Result<()> {
    let migrator = SwiftMigrator::new(from.to_string(), to.to_string(), dry_run);
    migrator.run(project_root)?;
    Ok(())
}

fn cmd_use(version: &str) -> Result<()> {
    println!("\n{}", "🔧 Configuring Swift Environment".bold());
    println!("{}", "=================================".bold());
    println!();
    println!("Target version: {}", version.cyan());
    println!();
    println!("{}", "Run these commands:".bold());
    println!();
    println!("  source <(foodshare-swift configure {} --export)", version);
    println!();
    println!("Or manually:");
    println!();

    cmd_configure(version, false)?;

    Ok(())
}

fn cmd_list() -> Result<()> {
    let active = SwiftToolchain::detect_active()?;
    let toolchains = SwiftToolchain::list_available()?;

    println!("\n{}", "📦 Available Swift Toolchains".bold());
    println!("{}", "============================".bold());
    println!();

    println!("{}", "Active:".bold());
    println!(
        "  {} {} {}",
        "→".green(),
        active.version.short_version().cyan().bold(),
        format!("({})", active.path.parent().unwrap().display()).dimmed()
    );
    println!();

    if !toolchains.is_empty() {
        println!("{}", "Installed:".bold());
        for tc in toolchains {
            let is_active = tc.path == active.path;
            let marker = if is_active { "→".green() } else { " ".normal() };
            let version_str = tc.version.short_version();
            let version = if is_active {
                version_str.cyan().bold().to_string()
            } else {
                version_str.to_string()
            };

            println!(
                "  {} {} {}",
                marker,
                version,
                format!(
                    "({})",
                    tc.path.parent().and_then(|p| p.parent()).unwrap().display()
                )
                .dimmed()
            );
        }
        println!();
    }

    println!("{}", "Usage:".bold());
    println!("  foodshare-swift use <version>");
    println!("  foodshare-swift configure <version> --export");
    println!();

    Ok(())
}
