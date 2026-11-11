//! `ghsync status` — the read-only drift table across every surface
//! (T12).
//!
//! One row per linked pair, per unlinked card, and per orphan issue.
//! The drift flags:
//!
//! - `unlinked` — a card with no linked issue.
//! - `orphan` — a `ghsync`-labeled issue no card links to.
//! - `label-mismatch` — the issue's label set differs from the
//!   projection (`ghsync` + `ghsync-<repo>`).
//! - `field-mismatch` — a project field value differs from ghsync's
//!   last written value (the cursor); someone moved the board by hand.
//! - `closed-but-not-done` — the issue is closed while the card is not
//!   done (the sanctioned human gesture; `push` blocks the card).
//! - `footer-missing` — the issue body lost its `kanban:` footer.
//! - `pr-evidence-missing` — a merged PR has no evidence comment yet.
//! - `feed-due` — a verdict/escalation exists that the feed has not
//!   posted yet (verdict newer than the last post).
//!
//! A clean pair (no flags) is `ok`; the report is `clean` when every
//! row is.

use serde::Serialize;
use thiserror::Error;

use crate::config::{Config, GHSYNC_LABEL, ProjConfig, Status, card_link_comment, labels_for};
use crate::discuss::{SYNC_PREFIX, verdict_for};
use crate::github::{GitHub, GitHubError};
use crate::kanban::{CardView, Kanban, KanbanError, LinkedIssue};
use crate::link::LinkMap;
use crate::pr::extract_pr_url;
use crate::push::{Cursor, normalize_value};
use crate::setup::HANDOFFS_CATEGORY;

/// A failure of the status step.
#[derive(Debug, Error)]
pub enum StatusError {
    #[error(transparent)]
    Kanban(#[from] KanbanError),
    #[error(transparent)]
    GitHub(#[from] GitHubError),
    /// The stateless link resolution reports via `anyhow`.
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

/// One row of the drift table.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatusRow {
    /// Card id, or `-` for orphan issues.
    pub task: String,
    /// Kanban status, or `-`.
    pub status: String,
    /// `owner/repo#n`, or `-` for unlinked cards.
    pub target: String,
    /// What the target is: `issue` or `-`.
    pub kind: String,
    /// Current issue labels (empty for card-only rows).
    pub labels: Vec<String>,
    /// Current project Status field value (or `-`).
    pub status_field: String,
    /// Current project Assignee field value (or `-`).
    pub assignee_field: String,
    /// Current project Phase field value (or `-`).
    pub phase_field: String,
    /// Current milestone title (or `-`).
    pub milestone: String,
    /// The card's PR, if any: `#n merged` / `#n open` / `-`.
    pub pr: String,
    /// The drift flags; empty when the pair is in sync.
    pub drift: Vec<String>,
    /// True when `drift` is empty.
    pub clean: bool,
}

impl StatusRow {
    fn finish(mut self) -> Self {
        self.clean = self.drift.is_empty();
        self
    }

    /// Whether the pair is in sync.
    pub fn ok(&self) -> bool {
        self.clean
    }
}

/// The drift table.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StatusReport {
    pub rows: Vec<StatusRow>,
    /// True when every row is clean.
    pub clean: bool,
}

impl StatusReport {
    /// The aligned human-readable table.
    pub fn display(&self) -> String {
        let header: [&str; 10] = [
            "task",
            "status",
            "target",
            "labels",
            "Status",
            "Assignee",
            "Phase",
            "milestone",
            "pr",
            "drift",
        ];
        let mut rows: Vec<Vec<String>> = vec![header.iter().map(|h| h.to_string()).collect()];
        for row in &self.rows {
            let cells = [
                &row.task,
                &row.status,
                &row.target,
                &row.labels.join(","),
                &row.status_field,
                &row.assignee_field,
                &row.phase_field,
                &row.milestone,
                &row.pr,
                &row.drift.join(","),
            ];
            rows.push(
                cells
                    .iter()
                    .map(|c| if c.is_empty() { "-" } else { c })
                    .map(ToString::to_string)
                    .collect(),
            );
        }
        let widths: Vec<usize> = (0..10)
            .map(|i| rows.iter().map(|r| r[i].len()).max().unwrap_or(0))
            .collect();
        let mut out = String::new();
        for row in &rows {
            let line = row
                .iter()
                .enumerate()
                .map(|(i, c)| format!("{c:<w$}", w = widths[i]))
                .collect::<Vec<_>>()
                .join("  ");
            out.push_str(line.trim_end());
            out.push('\n');
        }
        out
    }
}

/// Run the read-only drift scan.
pub async fn run_status(
    kb: &mut impl Kanban,
    gh: &mut impl GitHub,
    cfg: &Config,
    cursor: &Cursor,
) -> Result<StatusReport, StatusError> {
    let cards = kb.list().await?;
    let mut views = Vec::with_capacity(cards.len());
    for card in &cards {
        views.push(kb.show(&card.id).await?);
    }
    let repos: Vec<String> = cfg.repos.iter().map(|r| r.name.clone()).collect();
    let links = crate::link::resolve_links_views(gh, &cfg.board, &cfg.org, &repos, &views).await?;

    let mut rows = Vec::new();
    let mut sorted: Vec<&CardView> = views.iter().collect();
    sorted.sort_by(|a, b| a.card.id.cmp(&b.card.id));
    for view in sorted {
        rows.push(card_row(view, gh, cfg, cursor, &links).await?);
    }

    // Orphan issues: `ghsync`-labeled, linked to no card.
    for repo in &repos {
        for iss in gh.issues_labeled(repo, GHSYNC_LABEL).await? {
            let key = LinkedIssue {
                owner: cfg.org.clone(),
                repo: repo.clone(),
                number: iss.number,
            };
            if links.by_issue.contains_key(&key) {
                continue;
            }
            rows.push(StatusRow {
                task: "-".into(),
                status: "-".into(),
                target: format!("{}/{}#{}", cfg.org, repo, iss.number),
                kind: "issue".into(),
                labels: iss.labels.clone(),
                status_field: "-".into(),
                assignee_field: "-".into(),
                phase_field: "-".into(),
                milestone: iss.milestone.clone().unwrap_or_else(|| "-".into()),
                pr: "-".into(),
                drift: vec!["orphan".into()],
                clean: false,
            });
        }
    }

    let clean = rows.iter().all(|r| r.clean);
    Ok(StatusReport { rows, clean })
}

async fn card_row(
    view: &CardView,
    gh: &mut impl GitHub,
    cfg: &Config,
    cursor: &Cursor,
    links: &LinkMap,
) -> Result<StatusRow, StatusError> {
    let card = &view.card;
    let Some(linked) = links.issue_for_card(&card.id).cloned() else {
        return Ok(StatusRow {
            task: card.id.clone(),
            status: card.status.clone(),
            target: "-".into(),
            kind: "-".into(),
            labels: Vec::new(),
            status_field: "-".into(),
            assignee_field: "-".into(),
            phase_field: "-".into(),
            milestone: "-".into(),
            pr: "-".into(),
            drift: vec!["unlinked".into()],
            clean: false,
        }
        .finish());
    };

    let mut drift = Vec::new();
    let issue = gh.get_issue(&linked.repo, linked.number).await?;

    // Label mismatch: the projection owns the label set.
    let expected = labels_for(&linked.repo);
    if issue.labels != expected {
        drift.push("label-mismatch".into());
    }

    // Footer missing: the issue body must carry the kanban footer.
    let footer = card_link_comment(&cfg.board, &card.id);
    if !issue.body.lines().any(|l| l.trim() == footer) {
        drift.push("footer-missing".into());
    }

    // Closed issue, open card: the sanctioned gesture push will honor.
    if issue.state == "closed"
        && !matches!(
            card.status_enum(),
            Some(Status::Done) | Some(Status::Archived)
        )
    {
        drift.push("closed-but-not-done".into());
    }

    // Project fields (display + cursor comparison).
    let mut status_field = String::from("-");
    let mut assignee_field = String::from("-");
    let mut phase_field = String::from("-");
    if let Some(proj) = cfg.project.as_ref()
        && !proj.project_id.is_empty()
    {
        (status_field, assignee_field, phase_field) =
            read_project_fields(gh, proj, &linked, cursor, &mut drift).await?;
    }

    // PR evidence.
    let mut pr = String::from("-");
    let text = card
        .completion_contract
        .as_deref()
        .map(|c| format!("{c}\n{}", card.body))
        .unwrap_or_else(|| card.body.clone());
    if let Some((owner, repo, number)) = extract_pr_url(&text)
        && owner == cfg.org
        && cfg.repo(&repo).is_some()
    {
        let pull = gh.get_pull(&repo, number).await?;
        pr = if pull.merged {
            format!("#{number} merged")
        } else {
            format!("#{number} open")
        };
        if pull.merged {
            let comments = gh.issue_comments(&repo, linked.number).await?;
            let marker = format!("PR #{number} merged");
            let has_evidence = comments
                .iter()
                .any(|c| c.body.lines().any(|l| l.trim().starts_with(&marker)));
            if !has_evidence {
                drift.push("pr-evidence-missing".into());
            }
        }
    }

    // Discussion feed: a verdict the feed has not posted yet.
    if let Some(verdict) = verdict_for(view)
        && let Some(repo_cfg) = cfg.repo(&linked.repo)
        && repo_cfg.discussions_enabled
    {
        let discussions = gh
            .list_discussions(&linked.repo, Some(HANDOFFS_CATEGORY), None)
            .await?;
        let card_ref = format!("Card: kanban:{}:{}", cfg.board, card.id);
        let verdict_line = format!("{SYNC_PREFIX}{verdict}");
        let posted = discussions.iter().any(|d| {
            d.body.lines().any(|l| l.trim() == card_ref)
                && (d.body.lines().any(|l| l.trim() == verdict_line)
                    || d.comments.iter().any(|c| c == verdict_line.as_str()))
        });
        if !posted {
            drift.push("feed-due".into());
        }
    }

    let row = StatusRow {
        task: card.id.clone(),
        status: card.status.clone(),
        target: linked.key(),
        kind: "issue".into(),
        labels: issue.labels.clone(),
        status_field,
        assignee_field,
        phase_field,
        milestone: issue.milestone.clone().unwrap_or_else(|| "-".into()),
        pr,
        drift,
        clean: false,
    };
    Ok(row.finish())
}

/// Read the three project fields of a linked issue: display values plus
/// the cursor comparison. A field that differs from ghsync's last
/// written value (the board moved by hand) records `field-mismatch`
/// once. A field the cursor has never written is pending push, not
/// drift.
async fn read_project_fields(
    gh: &mut impl GitHub,
    proj: &ProjConfig,
    linked: &LinkedIssue,
    cursor: &Cursor,
    drift: &mut Vec<String>,
) -> Result<(String, String, String), GitHubError> {
    let mut status_field = String::from("-");
    let mut assignee_field = String::from("-");
    let mut phase_field = String::from("-");
    let Some(item_id) = gh
        .project_item_for_issue(&proj.project_id, &linked.repo, linked.number)
        .await?
    else {
        return Ok((status_field, assignee_field, phase_field));
    };
    let issue_key = linked.key();
    for (field_name, field_id) in [
        ("Status", proj.status_field.as_str()),
        ("Assignee", proj.assignee_field.as_str()),
        ("Phase", proj.phase_field.as_str()),
    ] {
        let Some(raw) = gh.project_field_value(&item_id, field_id).await? else {
            continue;
        };
        match field_name {
            "Status" => status_field = raw.clone(),
            "Assignee" => assignee_field = raw.clone(),
            _ => phase_field = raw.clone(),
        }
        if let Some(last) = cursor.written(&issue_key, field_name)
            && normalize_value(field_name, &raw, proj) != last
        {
            drift.push("field-mismatch".into());
            break;
        }
    }
    Ok((status_field, assignee_field, phase_field))
}
