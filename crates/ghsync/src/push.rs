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

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::{
    Config, ProjConfig, Status, card_link_comment, issue_link_comment, labels_for, phase_from_card,
};
use crate::github::{GitHub, Milestone};
use crate::kanban::{CardView, Kanban};
use crate::link::{LinkMap, card_repo};

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

// ---------- part 2: project fields ----------

/// Last-written project field values, per issue. The only on-disk state
/// ghsync keeps. A missing or corrupt file degrades to "first sight" for
/// every pair (one extra idempotent read each), never to a wrong write.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Cursor {
    /// "owner/repo#n" -> field name -> last written value (the single-
    /// select option id, or the text value).
    #[serde(default)]
    pub fields: BTreeMap<String, BTreeMap<String, String>>,
}

impl Cursor {
    /// Load from `path`; missing or corrupt files start empty.
    pub fn load(path: &Path) -> Cursor {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    /// Save atomically (temp file + rename) so a killed run can never
    /// leave a torn cursor behind.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent()
            && !dir.as_os_str().is_empty()
        {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// The last written value for (issue, field), if any.
    pub fn written(&self, issue_key: &str, field: &str) -> Option<&str> {
        self.fields
            .get(issue_key)
            .and_then(|f| f.get(field))
            .map(String::as_str)
    }

    /// Record a last-written value.
    pub fn set(&mut self, issue_key: &str, field: &str, value: &str) {
        self.fields
            .entry(issue_key.to_string())
            .or_default()
            .insert(field.to_string(), value.to_string());
    }
}

/// One project field transition.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldWrite {
    /// "owner/repo#n".
    pub issue: String,
    /// "Status" | "Assignee" | "Phase".
    pub field: String,
    /// The value before this run's write; `None` on first sight when the
    /// project had no value.
    pub from: Option<String>,
    /// The value written (option id or text).
    pub to: String,
}

/// The outcome of the project-fields half of a push.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectReport {
    /// Field writes actually applied.
    pub writes: Vec<FieldWrite>,
    /// Transitions a dry run would apply.
    pub would_write: Vec<FieldWrite>,
    /// Issues added to the project this run ("owner/repo#n").
    pub items_added: Vec<String>,
    /// Drift rows: `setup-required`, `unknown-status-option`,
    /// `unknown-assignee`, `unknown-phase-option`.
    pub drift: Vec<Drift>,
    /// Cards not processed: (task_id, reason).
    pub skipped: Vec<(String, String)>,
}

/// Push the project fields (Status/Assignee/Phase) for every linked card
/// (part 2).
///
/// Dedupe: a field is written only when the computed value differs from
/// the cursor's last-written value. First sight of a pair (no cursor
/// entry) reads the current value once via the GraphQL node query and
/// records it — a matching read costs no write. Steady state therefore
/// costs zero GraphQL calls at all.
pub async fn push_project_fields(
    gh: &mut impl GitHub,
    cfg: &Config,
    views: &[CardView],
    links: &LinkMap,
    cursor: &mut Cursor,
    dry_run: bool,
) -> Result<ProjectReport> {
    let mut report = ProjectReport::default();
    let Some(proj) = cfg.project.as_ref() else {
        return Ok(report);
    };
    // Without discovered ids (T8 setup: project + option ids) no field
    // can be written.
    let setup_done = !proj.project_id.is_empty()
        && !proj.status_option_ids.is_empty()
        && !proj.phase_option_ids.is_empty();
    let project_id = proj.project_id.clone();

    let mut sorted: Vec<&CardView> = views.iter().collect();
    sorted.sort_by(|a, b| a.card.id.cmp(&b.card.id));

    for view in sorted {
        let card = &view.card;
        let Some(linked) = links.issue_for_card(&card.id) else {
            report.skipped.push((card.id.clone(), "unlinked".into()));
            continue;
        };
        let status = match Status::from_kanban(&card.status) {
            Some(s) => s,
            None => {
                report
                    .skipped
                    .push((card.id.clone(), "unknown-status".into()));
                continue;
            }
        };
        if status == Status::Archived {
            report.skipped.push((card.id.clone(), "archived".into()));
            continue;
        }
        let issue_key = linked.key();
        if !setup_done {
            report.drift.push(Drift {
                card: card.id.clone(),
                kind: "setup-required".into(),
                detail: format!("{issue_key}: no option ids in config; run `ghsync setup`"),
            });
            continue;
        }

        // Desired values: field name -> (cursor value, GraphQL value).
        let mut desired: BTreeMap<String, (String, serde_json::Value)> = BTreeMap::new();
        let status_label = status.project_status();
        match proj.status_option_ids.get(status_label) {
            Some(opt) => {
                desired.insert(
                    "Status".into(),
                    (
                        opt.clone(),
                        serde_json::json!({ "singleSelectOptionId": opt }),
                    ),
                );
            }
            None => report.drift.push(Drift {
                card: card.id.clone(),
                kind: "unknown-status-option".into(),
                detail: format!("{issue_key}: no option id for `{status_label}`"),
            }),
        }
        let profile = card
            .assignee
            .clone()
            .unwrap_or_else(|| cfg.default_assignee.clone());
        match proj
            .assignee_options
            .iter()
            .find(|(_, p)| p.as_str() == profile)
        {
            Some((label, _)) => {
                desired.insert(
                    "Assignee".into(),
                    (label.clone(), serde_json::json!({ "text": label })),
                );
            }
            None => report.drift.push(Drift {
                card: card.id.clone(),
                kind: "unknown-assignee".into(),
                detail: format!("{issue_key}: no project option for profile `{profile}`"),
            }),
        }
        let phase_slug = phase_from_card(status, card.assignee.as_deref());
        let phase_ok = proj
            .phase_options
            .iter()
            .find(|(_, s)| s.as_str() == phase_slug)
            .and_then(|(label, _)| proj.phase_option_ids.get(label))
            .map(|opt| {
                desired.insert(
                    "Phase".into(),
                    (
                        opt.clone(),
                        serde_json::json!({ "singleSelectOptionId": opt }),
                    ),
                )
            })
            .is_some();
        if !phase_ok {
            report.drift.push(Drift {
                card: card.id.clone(),
                kind: "unknown-phase-option".into(),
                detail: format!("{issue_key}: no option id for phase `{phase_slug}`"),
            });
        }

        let mut item_id: Option<String> = None;
        for (field_name, (stored, value)) in &desired {
            let field_id = match field_name.as_str() {
                "Status" => proj.status_field.as_str(),
                "Assignee" => proj.assignee_field.as_str(),
                _ => proj.phase_field.as_str(),
            };
            let last = cursor.written(&issue_key, field_name);
            if last == Some(stored.as_str()) {
                // Steady state: no read, no write.
                continue;
            }
            if dry_run {
                report.would_write.push(FieldWrite {
                    issue: issue_key.clone(),
                    field: field_name.clone(),
                    from: last.map(str::to_string),
                    to: stored.clone(),
                });
                continue;
            }
            let item = match &item_id {
                Some(id) => id.clone(),
                None => {
                    let id = match gh
                        .project_item_for_issue(&project_id, &linked.repo, linked.number)
                        .await?
                    {
                        Some(id) => id,
                        None => {
                            // Not in the project yet: fetch the issue for
                            // its node id, then add it.
                            let issue = gh.get_issue(&linked.repo, linked.number).await?;
                            let id = gh.add_project_item(&project_id, &issue.node_id).await?;
                            report.items_added.push(issue_key.clone());
                            id
                        }
                    };
                    item_id = Some(id.clone());
                    id
                }
            };
            let from = if last.is_none() {
                // First sight: read the current value once.
                let current = gh.project_field_value(&item, field_id).await?;
                match current {
                    Some(c) => {
                        let normalized = normalize_value(field_name, &c, proj);
                        if normalized == *stored {
                            // Board and project already agree; record
                            // without a write.
                            cursor.set(&issue_key, field_name, stored);
                            continue;
                        }
                        Some(normalized)
                    }
                    // The project has no value yet: write it.
                    None => None,
                }
            } else {
                last.map(str::to_string)
            };
            gh.set_project_field(&item, field_id, value).await?;
            cursor.set(&issue_key, field_name, stored);
            report.writes.push(FieldWrite {
                issue: issue_key.clone(),
                field: field_name.clone(),
                from,
                to: stored.clone(),
            });
        }
    }
    Ok(report)
}

/// A field value read back from the project, normalized to the cursor's
/// vocabulary: single-select readbacks are option names (labels) and map
/// to their option id; anything else (text values, already-id values)
/// compares as-is.
pub fn normalize_value(field: &str, raw: &str, proj: &ProjConfig) -> String {
    let ids = match field {
        "Status" => Some(&proj.status_option_ids),
        "Phase" => Some(&proj.phase_option_ids),
        _ => None,
    };
    match ids.and_then(|m| m.get(raw)) {
        Some(id) => id.clone(),
        None => raw.to_string(),
    }
}
