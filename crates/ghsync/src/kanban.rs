//! The kanban seam: what ghsync can do to the board.
//!
//! [`Kanban`] is the only interface ghsync has to the board. [`RealKanban`]
//! implements it by shelling out to the `hermes kanban` CLI (the board's
//! only sanctioned interface); [`crate::fakes::FakeKanban`] is the in-memory
//! stand-in used by the tests.
//!
//! RealKanbin refuses to run in a fenced context: a `delegate_task` child
//! (the `HERMES_DELEGATED_CHILD_CONTEXT` marker) is denied board mutations
//! by the CLI, and ghsync fails fast with a clear error instead of a
//! confusing subprocess failure. Run ghsync from a plain terminal or cron.

use std::io::Write;
use std::path::PathBuf;

use thiserror::Error;

use crate::config::{Status, issue_link_comment, parse_issue_link, phase_from_card};

/// A GitHub issue this card is linked to: `owner/repo#number`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LinkedIssue {
    pub owner: String,
    pub repo: String,
    pub number: u32,
}

impl LinkedIssue {
    /// The issue-side link comment for this issue.
    pub fn link_comment(&self) -> String {
        issue_link_comment(&self.owner, &self.repo, self.number)
    }

    pub fn key(&self) -> String {
        format!("{}/{}#{}", self.owner, self.repo, self.number)
    }
}

/// A kanban card as ghsync needs it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Card {
    pub id: String,
    pub title: String,
    pub body: String,
    pub assignee: Option<String>,
    pub status: String,
    pub priority: i64,
    pub completion_contract: Option<String>,
    /// Issues this card is linked to (from its link comments).
    pub links: Vec<LinkedIssue>,
    pub labels: Vec<String>,
}

impl Card {
    /// The parsed status, if the string is a known kanban status.
    pub fn status_enum(&self) -> Option<Status> {
        Status::from_kanban(&self.status)
    }

    /// The `Phase` for this card (see `config::phase_from_card`).
    pub fn phase(&self) -> &'static str {
        match self.status_enum() {
            Some(status) => phase_from_card(status, self.assignee.as_deref()),
            None => "backlog",
        }
    }
}

/// Parse the link lines out of a card's comments.
pub fn links_from_comments(comments: &[String]) -> Vec<LinkedIssue> {
    comments
        .iter()
        .filter_map(|c| parse_issue_link(c))
        .map(|r| LinkedIssue {
            owner: r.owner,
            repo: r.repo,
            number: r.number,
        })
        .collect()
}

/// A card plus its comments (from `show`).
#[derive(Debug, Clone, Default)]
pub struct CardView {
    pub card: Card,
    pub comments: Vec<String>,
    pub children: Vec<String>,
    pub latest_summary: Option<String>,
}

/// The kanban surface ghsync talks to.
///
/// `async_fn_in_trait` is allowed deliberately: this is an internal trait
/// (the crate's own implementations and tests are the only callers), and
/// every implementation returns a `Send` future anyway.
#[allow(async_fn_in_trait)]
pub trait Kanban: Send + Sync {
    /// All cards, every status (the real list includes archived).
    async fn list(&mut self) -> Result<Vec<Card>, KanbanError>;
    /// A card plus its comments and children.
    async fn show(&mut self, task_id: &str) -> Result<CardView, KanbanError>;
    /// Create a card. `idempotency_key` makes retries safe: a second call
    /// with the same key returns the existing card instead of a duplicate.
    async fn create(
        &mut self,
        title: &str,
        body: &str,
        assignee: &str,
        idempotency_key: &str,
        initial_status: &str,
    ) -> Result<Card, KanbanError>;
    /// Append a comment to a card.
    async fn comment(&mut self, task_id: &str, text: &str) -> Result<(), KanbanError>;
    /// Block a card with a typed reason.
    async fn block(&mut self, task_id: &str, kind: &str, reason: &str) -> Result<(), KanbanError>;
    /// Unblock a card (back to ready).
    async fn unblock(&mut self, task_id: &str, reason: &str) -> Result<(), KanbanError>;
    /// Complete a card with a handoff summary.
    async fn complete(&mut self, task_id: &str, summary: &str) -> Result<(), KanbanError>;
}

/// Failure of a kanban operation.
#[derive(Debug, Error)]
pub enum KanbanError {
    #[error(
        "ghsync: the hermes kanban CLI is fenced in this context (delegate_task child); \
         run ghsync from a plain terminal or cron. {detail}"
    )]
    Fenced { detail: String },
    #[error("no card with id {0} on board {1}")]
    NotFound(String, String),
    #[error("hermes kanban {cmd} failed (exit {code}): {stderr}")]
    Command {
        cmd: String,
        code: i32,
        stderr: String,
    },
    #[error("hermes kanban {cmd} returned invalid JSON: {detail}")]
    BadJson { cmd: String, detail: String },
}

/// Whether the marker value means board access is fenced for this process.
///
/// The marker is the `HERMES_DELEGATED_CHILD_CONTEXT` env var: present and
/// non-empty in delegate_task children and supervised workers.
pub fn is_fenced(marker: Option<&str>) -> bool {
    marker.is_some_and(|m| !m.is_empty())
}

/// A kanban client that shells out to the `hermes kanban` CLI.
pub struct RealKanban {
    /// The `hermes` executable.
    pub bin: PathBuf,
    pub board: String,
    /// The `HERMES_DELEGATED_CHILD_CONTEXT` marker at construction time.
    fenced: bool,
}

impl RealKanban {
    /// Build from the process environment.
    pub fn new(board: &str) -> Self {
        let marker = std::env::var_os("HERMES_DELEGATED_CHILD_CONTEXT")
            .map(|m| m.to_string_lossy().into_owned());
        Self::with_marker(board, marker.as_deref())
    }

    /// Build with an explicit marker value (`None` = unset). Used by tests.
    pub fn with_marker(board: &str, marker: Option<&str>) -> Self {
        Self {
            bin: PathBuf::from("hermes"),
            board: board.to_string(),
            fenced: is_fenced(marker),
        }
    }

    fn fence_check(&self) -> Result<(), KanbanError> {
        if self.fenced {
            return Err(KanbanError::Fenced {
                detail: "HERMES_DELEGATED_CHILD_CONTEXT is set in this environment".into(),
            });
        }
        Ok(())
    }

    /// The full argv for a subcommand (board flag before the subcommand).
    pub fn argv(&self, subcommand: &[&str], json: bool) -> Vec<String> {
        let mut args = vec![
            "kanban".to_string(),
            "--board".to_string(),
            self.board.clone(),
        ];
        args.extend(subcommand.iter().map(|s| s.to_string()));
        if json {
            args.push("--json".to_string());
        }
        args
    }

    async fn run(
        &self,
        subcommand: &[&str],
        json: bool,
    ) -> Result<std::process::Output, KanbanError> {
        self.fence_check()?;
        let output = tokio::process::Command::new(&self.bin)
            .args(self.argv(subcommand, json))
            .output()
            .await
            .map_err(|e| KanbanError::Command {
                cmd: subcommand.join(" "),
                code: -1,
                stderr: e.to_string(),
            })?;
        Ok(output)
    }

    async fn run_json(&mut self, subcommand: &[&str]) -> Result<serde_json::Value, KanbanError> {
        let output = self.run(subcommand, true).await?;
        if !output.status.success() {
            return Err(self.map_failure(subcommand, &output));
        }
        parse_json(&output, subcommand)
    }
}

impl RealKanban {
    fn map_failure(&self, subcommand: &[&str], output: &std::process::Output) -> KanbanError {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let text = format!("{stderr}{stdout}");
        if text.contains("cannot mutate Kanban tasks or boards") {
            return KanbanError::Fenced {
                detail: "the CLI refused the command".into(),
            };
        }
        KanbanError::Command {
            cmd: subcommand.join(" "),
            code: output.status.code().unwrap_or(-1),
            stderr: text.trim().to_string(),
        }
    }
}

fn parse_json(
    output: &std::process::Output,
    subcommand: &[&str],
) -> Result<serde_json::Value, KanbanError> {
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    serde_json::from_str(&text).map_err(|e| KanbanError::BadJson {
        cmd: subcommand.join(" "),
        detail: e.to_string(),
    })
}

fn task_to_card(v: &serde_json::Value) -> Card {
    Card {
        id: v
            .get("id")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        title: v
            .get("title")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        body: v
            .get("body")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        assignee: v
            .get("assignee")
            .and_then(|x| x.as_str())
            .map(str::to_string),
        status: v
            .get("status")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        priority: v.get("priority").and_then(|x| x.as_i64()).unwrap_or(0),
        completion_contract: v
            .get("completion_contract")
            .and_then(|x| x.as_str())
            .map(str::to_string),
        links: vec![],
        labels: vec![],
    }
}

impl Kanban for RealKanban {
    async fn list(&mut self) -> Result<Vec<Card>, KanbanError> {
        let v = self.run_json(&["list"]).await?;
        let arr = v.as_array().ok_or_else(|| KanbanError::BadJson {
            cmd: "list".into(),
            detail: "expected a JSON array".into(),
        })?;
        Ok(arr.iter().map(task_to_card).collect())
    }

    async fn show(&mut self, task_id: &str) -> Result<CardView, KanbanError> {
        let args: Vec<&str> = vec!["show", task_id];
        let v = self.run_json(&args).await?;
        let task = v
            .get("task")
            .ok_or_else(|| KanbanError::NotFound(task_id.into(), self.board.clone()))?;
        let mut card = task_to_card(task);
        let comments: Vec<String> = v
            .get("comments")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c.get("body").and_then(|b| b.as_str()))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        card.links = links_from_comments(&comments);
        let children: Vec<String> = v
            .get("children")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c.as_str().or_else(|| c.get("id").and_then(|x| x.as_str())))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let latest_summary = v
            .get("latest_summary")
            .and_then(|s| s.as_str())
            .map(str::to_string);
        Ok(CardView {
            card,
            comments,
            children,
            latest_summary,
        })
    }

    async fn create(
        &mut self,
        title: &str,
        body: &str,
        assignee: &str,
        idempotency_key: &str,
        initial_status: &str,
    ) -> Result<Card, KanbanError> {
        // The body can be long and multi-line: pass it via a temp file so
        // shell quoting can't mangle it.
        let mut tmp = tempfile::NamedTempFile::new().map_err(|e| KanbanError::Command {
            cmd: "create".into(),
            code: -1,
            stderr: e.to_string(),
        })?;
        tmp.write_all(body.as_bytes())
            .map_err(|e| KanbanError::Command {
                cmd: "create".into(),
                code: -1,
                stderr: e.to_string(),
            })?;
        let tmp_path = tmp.path().to_path_buf();
        let body_file = tmp_path.to_string_lossy().into_owned();
        let args = vec![
            "create",
            title,
            "--body-file",
            body_file.as_str(),
            "--assignee",
            assignee,
            "--idempotency-key",
            idempotency_key,
            "--initial-status",
            initial_status,
        ];
        let v = self.run_json(&args).await?;
        let card = task_to_card(&v);
        drop(tmp);
        Ok(card)
    }

    async fn comment(&mut self, task_id: &str, text: &str) -> Result<(), KanbanError> {
        let args: Vec<&str> = vec!["comment", task_id, text];
        let output = self.run(&args, false).await?;
        if !output.status.success() {
            return Err(self.map_failure(&["comment"], &output));
        }
        Ok(())
    }

    async fn block(&mut self, task_id: &str, kind: &str, reason: &str) -> Result<(), KanbanError> {
        let args: Vec<&str> = vec!["block", task_id, reason, "--kind", kind];
        let output = self.run(&args, false).await?;
        if !output.status.success() {
            return Err(self.map_failure(&["block"], &output));
        }
        Ok(())
    }

    async fn unblock(&mut self, task_id: &str, reason: &str) -> Result<(), KanbanError> {
        let args: Vec<&str> = vec!["unblock", task_id, "--reason", reason];
        let output = self.run(&args, false).await?;
        if !output.status.success() {
            return Err(self.map_failure(&["unblock"], &output));
        }
        Ok(())
    }

    async fn complete(&mut self, task_id: &str, summary: &str) -> Result<(), KanbanError> {
        let args: Vec<&str> = vec!["complete", task_id, "--summary", summary];
        let output = self.run(&args, false).await?;
        if !output.status.success() {
            return Err(self.map_failure(&["complete"], &output));
        }
        Ok(())
    }
}
