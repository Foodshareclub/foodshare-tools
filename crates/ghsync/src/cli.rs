//! The ghsync command line.
//!
//! The command logic lives in the library so it can be exercised without
//! spawning a process. [`run`] takes the full argv and returns a result;
//! the binary in `main.rs` is a thin wrapper.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

use crate::kanban::Kanban;

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
            let cfg = crate::config::Config::load(&cli.config_path())?;
            let token = cfg.token.clone().ok_or_else(|| {
                anyhow::anyhow!(
                    "no token configured; set it in {}",
                    cli.config_path().display()
                )
            })?;
            let mut kb = crate::kanban::RealKanban::new(&cfg.board);
            let mut gh = crate::github::RealGitHub::new(&cfg.api, &cfg.org, &token);
            let cards = kb.list().await?;
            let mut views = Vec::with_capacity(cards.len());
            for card in &cards {
                views.push(kb.show(&card.id).await?);
            }
            let repos: Vec<String> = cfg.repos.iter().map(|r| r.name.clone()).collect();
            let links =
                crate::link::resolve_links_views(&mut gh, &cfg.board, &cfg.org, &repos, &views)
                    .await?;
            let report =
                crate::push::push_issues(&mut kb, &mut gh, &cfg, &views, &links, dry_run).await?;
            let mut cursor = crate::push::Cursor::load(&cfg.state_path);
            let proj_report = crate::push::push_project_fields(
                &mut gh,
                &cfg,
                &views,
                &links,
                &mut cursor,
                dry_run,
            )
            .await?;
            if !dry_run {
                cursor.save(&cfg.state_path)?;
            }
            for id in &report.would_create {
                println!("would create issue for {id}");
            }
            for id in &report.created {
                println!("created issue for {id}");
            }
            for g in &report.gestures {
                println!("{}: {} ({})", g.action, g.card, g.detail);
            }
            for d in &report.drift {
                println!("drift {}: {} ({})", d.kind, d.card, d.detail);
            }
            for (id, why) in &report.skipped {
                println!("skipped {id}: {why}");
            }
            for w in &proj_report.writes {
                println!(
                    "project {}: {} {} -> {}",
                    w.issue,
                    w.field,
                    w.from.as_deref().unwrap_or("<unset>"),
                    w.to
                );
            }
            for w in &proj_report.would_write {
                println!("would set project {}: {} -> {}", w.issue, w.field, w.to);
            }
            for id in &proj_report.items_added {
                println!("added to project: {id}");
            }
            for d in &proj_report.drift {
                println!("drift {}: {} ({})", d.kind, d.card, d.detail);
            }
            println!(
                "push: {} created, {} verified, {} gestured, {} field writes, {} drift, {} skipped",
                report.created.len(),
                report.verified.len(),
                report.gestures.len(),
                proj_report.writes.len(),
                report.drift.len() + proj_report.drift.len(),
                report.skipped.len()
            );
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
