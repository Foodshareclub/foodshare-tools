//! `ghsync` discussion feed + discussion intake (T11).
//!
//! **Feed** (kanban → discussions): a card carrying a `sync:github
//! <verdict>` comment (approve / request-changes) or blocked with an
//! `escalation:` reason gets one discussion in its repo's `Handoffs`
//! category, titled `[<verdict>] <card title>`. The card reference in
//! the body (`Card: kanban:<board>:<id>`) is the re-derivation key: a
//! later run finds the discussion by that line and appends a comment
//! instead of creating a second discussion.
//!
//! **Intake** (discussions → kanban): a discussion labeled `intake`
//! becomes one card (`ghd-<owner>-<repo>-<n>` idempotency key,
//! `ghsyncd …` link comment), and the label is removed so a retry
//! neither duplicates the card nor re-lists the discussion.
//!
//! Both sides only touch repos where setup recorded Discussions
//! enabled; intake additionally requires recorded label support.

use thiserror::Error;

use crate::config::{Config, discussion_link_comment, issue_link_comment};
use crate::github::{GitHub, GitHubError};
use crate::intake::{INTAKE_INITIAL_STATUS, INTAKE_LABEL, IntakeCard};
use crate::kanban::{CardView, Kanban, KanbanError};
use crate::link::LinkMap;
use crate::setup::HANDOFFS_CATEGORY;

/// The prefix of a verdict comment on a card.
pub const SYNC_PREFIX: &str = "sync:github ";
/// The prefix of an escalation block reason.
pub const ESCALATION_PREFIX: &str = "escalation:";

/// The verdict tokens ghsync posts on. Anything else after the prefix
/// is logged, not posted (the verdict vocabulary is deliberate).
pub fn verdict_from_line(line: &str) -> Option<&'static str> {
    let rest = line.trim().strip_prefix(SYNC_PREFIX)?;
    match rest.split_whitespace().next()? {
        "approve" => Some("approve"),
        "request-changes" => Some("request-changes"),
        _ => None,
    }
}

/// What a feed run did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FeedReport {
    /// Discussions created.
    pub created: u32,
    /// Comments appended to existing discussions.
    pub appended: u32,
    /// Cards with a verdict but a gated repo: (card id, reason).
    pub skipped: Vec<(String, String)>,
}

/// The verdict a card currently carries, if any: the latest
/// `sync:github <verdict>` comment, else an `escalation:` block reason
/// on a blocked card.
pub fn verdict_for(view: &CardView) -> Option<&'static str> {
    if let Some(v) = view
        .comments
        .iter()
        .rev()
        .find_map(|c| c.lines().find_map(verdict_from_line))
    {
        return Some(v);
    }
    if view.card.status == "blocked" {
        return view
            .comments
            .iter()
            .rev()
            .find_map(|c| {
                c.lines()
                    .find(|l| l.trim_start().starts_with(ESCALATION_PREFIX))
            })
            .map(|_| "escalation");
    }
    None
}

/// Run the feed over every linked card with a verdict.
pub async fn post_feed(
    gh: &mut impl GitHub,
    cfg: &Config,
    views: &[CardView],
    links: &LinkMap,
) -> Result<FeedReport, GitHubError> {
    let mut report = FeedReport::default();
    for view in views {
        let card = &view.card;
        let Some(linked) = links.issue_for_card(&card.id) else {
            // Not linked to an issue: no repo to post to.
            continue;
        };
        let Some(verdict) = verdict_for(view) else {
            continue;
        };
        // Only repos where setup recorded Discussions enabled.
        let Some(repo_cfg) = cfg.repo(&linked.repo) else {
            report
                .skipped
                .push((card.id.clone(), "unknown repo".into()));
            continue;
        };
        if !repo_cfg.discussions_enabled {
            report
                .skipped
                .push((card.id.clone(), "discussions disabled".into()));
            continue;
        }

        let title = format!("[{verdict}] {}", card.title);
        let card_ref = format!("Card: kanban:{}:{}", cfg.board, card.id);
        let discussions = gh
            .list_discussions(&linked.repo, Some(HANDOFFS_CATEGORY), None)
            .await?;
        if let Some(existing) = discussions
            .iter()
            .find(|d| d.body.lines().any(|l| l.trim() == card_ref))
        {
            // One discussion per card: append, never recreate. The
            // comment itself is the dedupe.
            let comment = format!("{SYNC_PREFIX}{verdict}");
            if !existing.comments.iter().any(|c| c == &comment) {
                gh.discussion_comment(&linked.repo, &existing.id, &comment)
                    .await?;
                report.appended += 1;
            }
        } else {
            let body = format!(
                "{SYNC_PREFIX}{verdict}\n\n{}\n\n{card_ref}",
                issue_link_comment(&linked.owner, &linked.repo, linked.number)
            );
            gh.create_discussion(&linked.repo, HANDOFFS_CATEGORY, &title, &body)
                .await?;
            report.created += 1;
        }
    }
    Ok(report)
}

/// A failure of the discussion intake step.
#[derive(Debug, Error)]
pub enum DiscussError {
    #[error(transparent)]
    Kanban(#[from] KanbanError),
    #[error(transparent)]
    GitHub(#[from] GitHubError),
}

/// What a discussion intake run did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiscussionIntakeReport {
    /// Cards created, in repo/discussion order.
    pub created: Vec<IntakeCard>,
    /// Repos or discussions seen but not created for: (ref, reason).
    pub skipped: Vec<(String, String)>,
    /// References that would be created (dry run only).
    pub would_create: Vec<String>,
}

/// Run intake over discussions labeled `intake` in every configured
/// repo (gated on setup-recorded Discussions + label support).
pub async fn intake_discussions(
    kb: &mut impl Kanban,
    gh: &mut impl GitHub,
    cfg: &Config,
    dry_run: bool,
) -> Result<DiscussionIntakeReport, DiscussError> {
    let mut report = DiscussionIntakeReport::default();
    for repo in &cfg.repos {
        if !repo.discussions_enabled {
            report
                .skipped
                .push((repo.name.clone(), "discussions disabled".into()));
            continue;
        }
        if !repo.discussion_labels_enabled {
            report
                .skipped
                .push((repo.name.clone(), "discussion labels unsupported".into()));
            continue;
        }
        let discussions = gh
            .list_discussions(&repo.name, None, Some(INTAKE_LABEL))
            .await?;
        for d in discussions {
            let reference = format!("{}/{}#{}", cfg.org, repo.name, d.number);
            if dry_run {
                report.would_create.push(reference.clone());
                continue;
            }
            let title = format!("{} (ghd {}/{}#{})", d.title, cfg.org, repo.name, d.number);
            let link = discussion_link_comment(&cfg.org, &repo.name, d.number);
            let mut body = d.body.clone();
            if !body.ends_with('\n') {
                body.push('\n');
            }
            body.push_str(&format!("\n{link}"));
            let key = format!("ghd-{}-{}-{}", cfg.org, repo.name, d.number);
            let card = kb
                .create(
                    &title,
                    &body,
                    &cfg.default_assignee,
                    &key,
                    INTAKE_INITIAL_STATUS,
                )
                .await?;
            kb.comment(&card.id, &link).await?;

            // Remove the intake label (by id) so the next run does not
            // re-list the discussion.
            if let Some(label) = d.labels.iter().find(|l| l.name == INTAKE_LABEL) {
                gh.remove_discussion_label(&repo.name, &d.id, &label.id)
                    .await?;
            }

            report.created.push(IntakeCard {
                issue: reference,
                card: card.id,
                profile: cfg.default_assignee.clone(),
                phase: crate::intake::PHASE_AD_HOC.to_string(),
            });
        }
    }
    Ok(report)
}
