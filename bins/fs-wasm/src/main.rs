//! fs-wasm — the WebAssembly build engine.
//!
//! Compiles the five published crates to WebAssembly and patches the
//! generated loaders so Next.js can find them.
//!
//! Usage:
//!   fs-wasm                  # build every crate
//!   fs-wasm --json           # machine-readable output
//!   fs-wasm --only geo       # build one crate

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use foodshare_wasm::{Layout, build, loader};

#[derive(Parser, Debug)]
#[command(
    name = "fs-wasm",
    version,
    about = "FoodShare WebAssembly build engine"
)]
struct Cli {
    /// The Rust workspace that holds the crates.
    #[arg(long)]
    tools_dir: Option<PathBuf>,

    /// The web repository the generated modules go into.
    #[arg(long)]
    web_dir: Option<PathBuf>,

    /// Build only this crate.
    #[arg(long)]
    only: Option<String>,

    /// Print the results as JSON.
    #[arg(long)]
    json: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let tools_dir = cli.tools_dir.unwrap_or_else(resolve_tools_dir);
    // The web repository is a sibling of the tools repository. Resolving
    // the tools directory first keeps a `..` out of the paths that are
    // printed and logged.
    let tools_dir = std::fs::canonicalize(&tools_dir).unwrap_or(tools_dir);
    let web_dir = cli
        .web_dir
        .unwrap_or_else(|| tools_dir.join("..").join("foodshare-web"));
    let web_dir = std::fs::canonicalize(&web_dir).unwrap_or(web_dir);

    let crates: Vec<loader::Crate> = match &cli.only {
        Some(name) => loader::CRATES
            .iter()
            .copied()
            .filter(|c| {
                c.name == name
                    || c.name.replace('-', "_") == *name
                    || c.name.strip_prefix("foodshare-") == Some(name.as_str())
            })
            .collect(),
        None => loader::CRATES.to_vec(),
    };

    if crates.is_empty() {
        anyhow::bail!("no crate matches `{}`", cli.only.unwrap_or_default());
    }

    let layout = Layout {
        tools_dir: tools_dir.clone(),
        web_dir: web_dir.clone(),
    };

    if !cli.json {
        println!("🦀 =========================================================");
        println!("🦀 FoodShare Rust -> WebAssembly (WASM) Build Engine");
        println!("🦀 =========================================================");
    }

    let outcomes = build::build_all(&layout, &crates).await;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&outcomes)?);
    } else {
        for outcome in &outcomes {
            if outcome.success {
                let size = outcome
                    .size_kb
                    .map(|kb| format!("{kb} KB"))
                    .unwrap_or_else(|| "size unknown".to_string());
                println!(
                    "✅ [{}] compiled in {}ms -> {} ({size}, wasm-opt -Oz)",
                    outcome.crate_name,
                    outcome.duration_ms,
                    outcome.out_dir.display()
                );
            } else {
                println!(
                    "❌ [{}] failed: {}",
                    outcome.crate_name,
                    outcome.error.as_deref().unwrap_or("unknown error")
                );
            }
        }
        println!("\n==========================================");
        if outcomes.iter().all(|o| o.success) {
            println!("🎉 ALL RUST WASM MODULES COMPILED 100% GREEN!");
        } else {
            println!("❌ Some WASM modules failed to compile.");
        }
        println!("==========================================\n");
    }

    if outcomes.iter().all(|o| o.success) {
        Ok(())
    } else {
        std::process::exit(1);
    }
}

/// The Rust workspace, from the current directory.
fn resolve_tools_dir() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.join("crates").is_dir() {
        return cwd;
    }
    let nested = cwd.join("foodshare-tools");
    if nested.join("crates").is_dir() {
        return nested;
    }
    cwd
}
