//! The ghsync command line.
//!
//! The command logic lives in the library so it can be exercised without
//! spawning a process. [`run`] takes the full argv and returns a result;
//! the binary in `main.rs` is a thin wrapper.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

/// Keep the Foodshareclub GitHub org in step with the Hermes Kanban board.
///
/// Kanban is the single source of state; GitHub is projection, intake,
/// feed and evidence only.
#[derive(Parser, Debug)]
#[command(name = "ghsync", version, about)]
pub struct Cli {
    /// Path to the ghsync configuration file.
    #[arg(long, global = true, default_value = "~/.config/ghsync/config.toml")]
    pub config: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Verify the token and idempotently provision the org (labels, project, fields, milestones, views).
    Setup,
    /// Project kanban state onto GitHub: link-or-create issues, milestones, project fields, PR evidence.
    Push {
        /// Print the plan without writing to GitHub or kanban.
        #[arg(long)]
        dry_run: bool,
    },
    /// Pull intake from GitHub: intake-form issues and `intake` discussions become kanban cards.
    Intake {
        /// Print the plan without writing to kanban.
        #[arg(long)]
        dry_run: bool,
    },
    /// Read-only drift table across every surface.
    Status {
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
}

impl Cli {
    /// The config path with a leading `~/` resolved against `$HOME`.
    pub fn config_path(&self) -> PathBuf {
        expand_tilde(&self.config)
    }
}

/// Replace a leading `~/` with `$HOME`; pass the path through otherwise.
pub fn expand_tilde(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    path.to_path_buf()
}

/// Parse the command line from the given argv (program name first).
pub fn parse(args: &[String]) -> Result<Cli> {
    Cli::try_parse_from(args).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Run the parsed command to completion.
pub async fn run(args: Vec<String>) -> Result<()> {
    let cli = parse(&args)?;
    match cli.command {
        Command::Setup => bail!("setup: not implemented yet"),
        Command::Push { dry_run } => {
            println!("push {dry_run}");
            Ok(())
        }
        Command::Intake { dry_run } => bail!("intake {dry_run}: not implemented yet"),
        Command::Status { json } => bail!("status {json}: not implemented yet"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_push() {
        let cli = parse(&argv(&["ghsync", "push"])).unwrap();
        match cli.command {
            Command::Push { dry_run } => assert!(!dry_run),
            other => panic!("expected push, got {other:?}"),
        }
    }

    #[test]
    fn parse_push_dry_run() {
        let cli = parse(&argv(&["ghsync", "push", "--dry-run"])).unwrap();
        match cli.command {
            Command::Push { dry_run } => assert!(dry_run),
            other => panic!("expected push, got {other:?}"),
        }
    }

    #[test]
    fn parse_all_subcommands() {
        assert!(matches!(
            parse(&argv(&["ghsync", "setup"])).unwrap().command,
            Command::Setup
        ));
        assert!(matches!(
            parse(&argv(&["ghsync", "intake", "--dry-run"]))
                .unwrap()
                .command,
            Command::Intake { dry_run: true }
        ));
        assert!(matches!(
            parse(&argv(&["ghsync", "status", "--json"]))
                .unwrap()
                .command,
            Command::Status { json: true }
        ));
    }

    #[test]
    fn missing_subcommand_rejected() {
        assert!(parse(&argv(&["ghsync"])).is_err());
    }
}
