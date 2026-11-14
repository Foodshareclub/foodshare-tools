//! `ghsync intake`: `intake`-labeled issues become kanban cards.
//!
//! The intake form (`.github/ISSUE_TEMPLATE/agent-intake.yml`, shipped
//! in T6) sets the `intake` label, so submitting the form IS the intake
//! trigger — no manual label step. Plain issues labeled `intake` by hand
//! keep working: missing form sections fall back to the defaults
//! (`Ad-hoc` phase / config `default_assignee` profile).
//!
//! For each open `intake` issue without the `ghsync` label:
//! parse the form sections, validate the profile against the config
//! `profiles` list (unknown → loud error naming the list, nothing
//! written), create the card with idempotency key
//! `gh-<owner>-<repo>-<n>` (so a retry never duplicates), then
//! relabel the issue (`-intake +ghsync`), set the repo's default
//! milestone (unless the phase is `Ad-hoc`), and post the stateless
//! link comment on the card.

use std::collections::BTreeMap;

use thiserror::Error;

use crate::config::{Config, GHSYNC_LABEL, issue_link_comment};
use crate::github::{GitHub, GitHubError};
use crate::kanban::{Kanban, KanbanError};

/// The issue label that marks an issue as intake.
pub const INTAKE_LABEL: &str = "intake";
/// Phase value meaning "no project milestone".
pub const PHASE_AD_HOC: &str = "Ad-hoc";
/// The initial status of intake cards (Q1: they default to `ready`).
pub const INTAKE_INITIAL_STATUS: &str = "ready";

/// A failure of the intake step.
#[derive(Debug, Error)]
pub enum IntakeError {
    /// The form named an agent profile that is not in the config.
    #[error(
        "unknown agent profile `{profile}` for {issue}; valid profiles (config `profiles`): {valid:?}"
    )]
    UnknownProfile {
        profile: String,
        issue: String,
        valid: Vec<String>,
    },
    #[error(transparent)]
    Kanban(#[from] KanbanError),
    #[error(transparent)]
    GitHub(#[from] GitHubError),
}

/// Parsed form fields. Missing sections stay `None`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormFields {
    pub phase: Option<String>,
    pub profile: Option<String>,
    pub acceptance: Option<String>,
}

/// A form heading line. Returns the label plus any value rendered on the
/// SAME line after it. GitHub renders inline field types (dropdown,
/// checkbox) as `**<label>:** <value>` and block types (textarea) as a
/// bare `**<label>:**` heading with the value on following lines — both
/// shapes must parse (the live E2E issue foodshare-tools#217 uses the
/// same-line shape).
fn form_heading(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("**")?;
    let (inner, tail) = rest.split_once(":**")?;
    if inner.is_empty() {
        return None;
    }
    Some((inner.to_string(), tail.trim().to_string()))
}

/// Parse the rendered form sections out of an issue body.
///
/// GitHub renders each form field as a `**<label>:**` heading line
/// followed by its value; a section's value runs to the next heading.
/// This is the documented rendering — T14's live E2E re-verifies the
/// exact text against a real form submission.
pub fn parse_form(body: &str) -> FormFields {
    let mut lines: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in body.lines() {
        if let Some((label, same_line)) = form_heading(line.trim()) {
            if !same_line.is_empty() {
                lines.entry(label.clone()).or_default().push(same_line);
            }
            lines.entry(label.clone()).or_default();
            current = Some(label);
            continue;
        }
        if let Some(label) = &current
            && let Some(v) = lines.get_mut(label)
        {
            v.push(line.to_string());
        }
    }
    let section = |label: &str| -> Option<String> {
        lines
            .get(label)
            .map(|ls| ls.join("\n").trim().to_string())
            .filter(|s| !s.is_empty())
    };
    FormFields {
        phase: section("Phase"),
        profile: section("Agent profile"),
        acceptance: section("Acceptance criteria"),
    }
}

/// One intake card created by a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntakeCard {
    /// `owner/repo#n`.
    pub issue: String,
    /// The created card's task id.
    pub card: String,
    /// The profile the card was assigned (form or config default).
    pub profile: String,
    /// The phase (form or `Ad-hoc`).
    pub phase: String,
}

/// What an intake run did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntakeReport {
    /// Cards created, in repo/issue order.
    pub created: Vec<IntakeCard>,
    /// Issues seen but not created for: (issue, reason).
    pub skipped: Vec<(String, String)>,
    /// Issues that would be created (dry run only).
    pub would_create: Vec<String>,
}

/// Run intake over every configured repo.
pub async fn intake_issues(
    kb: &mut impl Kanban,
    gh: &mut impl GitHub,
    cfg: &Config,
    dry_run: bool,
) -> Result<IntakeReport, IntakeError> {
    let mut report = IntakeReport::default();
    for repo in &cfg.repos {
        let issues = gh.issues_labeled(&repo.name, INTAKE_LABEL).await?;
        for issue in issues {
            let issue_ref = format!("{}/{}#{}", cfg.org, repo.name, issue.number);
            if issue.state != "open" {
                report
                    .skipped
                    .push((issue_ref.clone(), "issue closed".into()));
                continue;
            }
            if issue.labels.iter().any(|l| l == GHSYNC_LABEL) {
                report
                    .skipped
                    .push((issue_ref.clone(), "already linked (ghsync label)".into()));
                continue;
            }

            let fields = parse_form(&issue.body);
            let phase = fields
                .phase
                .clone()
                .unwrap_or_else(|| PHASE_AD_HOC.to_string());
            let profile = fields
                .profile
                .clone()
                .unwrap_or_else(|| cfg.default_assignee.clone());
            if !cfg.profiles.iter().any(|p| p == &profile) {
                return Err(IntakeError::UnknownProfile {
                    profile,
                    issue: issue_ref,
                    valid: cfg.profiles.clone(),
                });
            }

            if dry_run {
                report.would_create.push(issue_ref.clone());
                continue;
            }

            let title = format!(
                "{} (gh {}/{}#{})",
                issue.title, cfg.org, repo.name, issue.number
            );
            let link = issue_link_comment(&cfg.org, &repo.name, issue.number);
            let mut body = issue.body.clone();
            if !body.ends_with('\n') {
                body.push('\n');
            }
            body.push_str(&format!("\n{link}"));
            let key = format!("gh-{}-{}-{}", cfg.org, repo.name, issue.number);
            let card = kb
                .create(&title, &body, &profile, &key, INTAKE_INITIAL_STATUS)
                .await?;
            kb.comment(&card.id, &link).await?;

            // Relabel: -intake +ghsync (other labels preserved).
            let mut labels: Vec<String> = issue
                .labels
                .iter()
                .filter(|l| l.as_str() != INTAKE_LABEL)
                .cloned()
                .collect();
            if !labels.iter().any(|l| l == GHSYNC_LABEL) {
                labels.push(GHSYNC_LABEL.to_string());
            }
            gh.set_labels(&repo.name, issue.number, &labels).await?;

            // Milestone: the repo's default, unless the phase is Ad-hoc.
            if phase != PHASE_AD_HOC {
                match gh
                    .list_milestones(&repo.name)
                    .await?
                    .into_iter()
                    .find(|m| m.title == repo.default_milestone)
                {
                    Some(ms) => {
                        gh.set_milestone(&repo.name, issue.number, ms.number)
                            .await?;
                    }
                    None => report.skipped.push((
                        issue_ref.clone(),
                        format!(
                            "milestone `{}` not found; run `ghsync setup`",
                            repo.default_milestone
                        ),
                    )),
                }
            }

            report.created.push(IntakeCard {
                issue: issue_ref.clone(),
                card: card.id.clone(),
                profile,
                phase,
            });
        }
    }
    Ok(report)
}
