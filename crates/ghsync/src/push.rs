//! Push (part 1): card → issue projection.
//!
//! Kanban is the single source of state. For each card in `triage..done`
//! (archived is skipped):
//!
//! * **unlinked** → create the issue (card body + link footer, the ghsync
//!   labels, the repo's default milestone) and post the stateless link
//!   comment on the card;
//! * **linked** → ensure the footer is present in the issue body (a
//!   stripped footer is re-appended + a note comment), normalize the
//!   labels, enforce the repo's default milestone, and apply the state
//!   gestures:
//!   - card done + issue open → close the issue;
//!   - card active + issue closed → block the card (a human closed it);
//!   - a card blocked for that reason + issue reopened → unblock.
//!
//! A dry run records what would happen and writes nothing.

use crate::config::{Config, Status, card_link_comment, issue_link_comment, labels_for};
use crate::github::{GitHub, Milestone};
use crate::kanban::{CardView, Kanban};
use crate::link::{LinkMap, card_repo};
use anyhow::Result;

/// The exact reason push blocks a card when a human closes its issue.
pub const CLOSED_REASON: &str = "issue closed on GitHub (ghsync)";
/// The exact reason push unblocks a card when the issue is reopened.
pub const REOPEN_REASON: &str = "issue reopened on GitHub (ghsync)";
/// The note comment posted when a stripped footer is re-appended.
pub const FOOTER_NOTE: &str = "ghsync: re-appended kanban footer";

/// One state gesture applied to the board or to GitHub.
#[derive(Debug, Clone, PartialEq)]
pub struct Gesture {
    pub card: String,
    /// `close` | `block` | `unblock`.
    pub action: String,
    pub detail: String,
}

/// A drift row: the GitHub side diverged, or a card could not be
/// projected. Drift is reported, never repaired silently beyond the
/// gestures above.
#[derive(Debug, Clone, PartialEq)]
pub struct Drift {
    pub card: String,
    /// `unknown-repo` | `unknown-status` | `issue-missing` |
    /// `footer-restored` | `issue-closed-by-human`.
    pub kind: String,
    pub detail: String,
}

/// The outcome of one push.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PushReport {
    /// Cards whose issues were created this run.
    pub created: Vec<String>,
    /// Cards whose linked issues were verified (and repaired if needed).
    pub verified: Vec<String>,
    /// Cards that would get an issue in a dry run.
    pub would_create: Vec<String>,
    /// State gestures applied.
    pub gestures: Vec<Gesture>,
    /// Drift rows.
    pub drift: Vec<Drift>,
    /// Skipped cards: (task_id, reason).
    pub skipped: Vec<(String, String)>,
}

/// Run one push over the given card views. See the module docs.
pub async fn push_issues(
    kb: &mut impl Kanban,
    gh: &mut impl GitHub,
    cfg: &Config,
    views: &[CardView],
    links: &LinkMap,
    dry_run: bool,
) -> Result<PushReport> {
    let repos: Vec<String> = cfg.repos.iter().map(|r| r.name.clone()).collect();
    let mut report = PushReport::default();

    // Deterministic order: by card id.
    let mut sorted: Vec<&CardView> = views.iter().collect();
    sorted.sort_by(|a, b| a.card.id.cmp(&b.card.id));

    for view in sorted {
        let card = &view.card;
        let status = match Status::from_kanban(&card.status) {
            Some(s) => s,
            None => {
                report
                    .skipped
                    .push((card.id.clone(), "unknown-status".into()));
                report.drift.push(Drift {
                    card: card.id.clone(),
                    kind: "unknown-status".into(),
                    detail: card.status.clone(),
                });
                continue;
            }
        };
        if status == Status::Archived {
            report.skipped.push((card.id.clone(), "archived".into()));
            continue;
        }
        let Some(repo_name) = card_repo(card, &repos) else {
            report
                .skipped
                .push((card.id.clone(), "unknown-repo".into()));
            report.drift.push(Drift {
                card: card.id.clone(),
                kind: "unknown-repo".into(),
                detail: card.body.clone(),
            });
            continue;
        };
        // card_repo only returns configured repos.
        let repo_cfg = cfg.repo(repo_name).expect("configured repo");
        let footer = card_link_comment(&cfg.board, &card.id);

        match links.issue_for_card(&card.id).cloned() {
            None => {
                if dry_run {
                    report.would_create.push(card.id.clone());
                    continue;
                }
                let body = if card.body.trim().is_empty() {
                    format!("---\n{footer}")
                } else {
                    format!("{}\n\n---\n{}", card.body.trim_end(), footer)
                };
                let issue = gh
                    .create_issue(repo_name, &card.title, &body, &labels_for(repo_name))
                    .await?;
                let m = ensure_milestone(gh, repo_name, &repo_cfg.default_milestone).await?;
                gh.set_milestone(repo_name, issue.number, m.number).await?;
                kb.comment(
                    &card.id,
                    &issue_link_comment(&cfg.org, repo_name, issue.number),
                )
                .await?;
                report.created.push(card.id.clone());
            }
            Some(linked) => {
                let issue = match gh.get_issue(repo_name, linked.number).await {
                    Ok(i) => i,
                    Err(_) => {
                        report
                            .skipped
                            .push((card.id.clone(), "issue-missing".into()));
                        report.drift.push(Drift {
                            card: card.id.clone(),
                            kind: "issue-missing".into(),
                            detail: linked.key(),
                        });
                        continue;
                    }
                };

                // 1. The footer must live in the body.
                if !issue.body.lines().any(|l| l.trim() == footer) {
                    if dry_run {
                        report.drift.push(Drift {
                            card: card.id.clone(),
                            kind: "footer-restored".into(),
                            detail: format!("dry-run: footer missing ({})", linked.key()),
                        });
                    } else {
                        gh.set_issue_body(
                            repo_name,
                            issue.number,
                            &format!("{}\n\n---\n{}", issue.body.trim_end(), footer),
                        )
                        .await?;
                        gh.comment(repo_name, issue.number, FOOTER_NOTE).await?;
                        report.drift.push(Drift {
                            card: card.id.clone(),
                            kind: "footer-restored".into(),
                            detail: linked.key(),
                        });
                    }
                }

                // 2. Normalize labels to the ghsync set for the repo.
                let labels = labels_for(repo_name);
                if issue.labels != labels && !dry_run {
                    gh.set_labels(repo_name, issue.number, &labels).await?;
                }

                // 3. Enforce the repo's default milestone.
                if issue.milestone.as_deref() != Some(repo_cfg.default_milestone.as_str())
                    && !dry_run
                {
                    let m = ensure_milestone(gh, repo_name, &repo_cfg.default_milestone).await?;
                    gh.set_milestone(repo_name, issue.number, m.number).await?;
                }

                // 4. State gestures.
                let closed = issue.state == "closed";
                let blocked_here = card.status == "blocked"
                    && view.comments.iter().any(|c| c.contains(CLOSED_REASON));
                match (status, closed) {
                    (Status::Done, false) => {
                        if !dry_run {
                            gh.close_issue(repo_name, issue.number).await?;
                        }
                        report.gestures.push(Gesture {
                            card: card.id.clone(),
                            action: "close".into(),
                            detail: linked.key(),
                        });
                    }
                    (
                        Status::Todo
                        | Status::Triage
                        | Status::Ready
                        | Status::Running
                        | Status::Review
                        | Status::Blocked
                        | Status::Scheduled,
                        true,
                    ) if !blocked_here => {
                        // A human closed the issue; that is the sanctioned
                        // gesture for "this work is on hold".
                        if !dry_run {
                            kb.block(&card.id, "dependency", CLOSED_REASON).await?;
                        }
                        report.gestures.push(Gesture {
                            card: card.id.clone(),
                            action: "block".into(),
                            detail: CLOSED_REASON.into(),
                        });
                        report.drift.push(Drift {
                            card: card.id.clone(),
                            kind: "issue-closed-by-human".into(),
                            detail: linked.key(),
                        });
                    }
                    (_, false) if blocked_here => {
                        if !dry_run {
                            kb.unblock(&card.id, REOPEN_REASON).await?;
                        }
                        report.gestures.push(Gesture {
                            card: card.id.clone(),
                            action: "unblock".into(),
                            detail: REOPEN_REASON.into(),
                        });
                    }
                    _ => {}
                }
                report.verified.push(card.id.clone());
            }
        }
    }
    Ok(report)
}

/// Find the milestone `title` in `repo`, creating it if absent.
async fn ensure_milestone(gh: &mut impl GitHub, repo: &str, title: &str) -> Result<Milestone> {
    if let Some(m) = gh
        .list_milestones(repo)
        .await?
        .into_iter()
        .find(|m| m.title == title)
    {
        return Ok(m);
    }
    Ok(gh.create_milestone(repo, title).await?)
}
