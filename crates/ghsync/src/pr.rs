//! `ghsync` PR evidence (plan T10).
//!
//! For linked cards whose completion contract or body carries a PR URL
//! (`https://github.com/<owner>/<repo>/pull/<n>`): if the PR is merged,
//! post exactly one comment on the linked issue:
//!
//! ```text
//! PR #<n> merged - CI: <success|failure|pending> - contract: <contract>
//! ```
//!
//! Dedupe is the comment itself: an identical comment already on the
//! issue skips the post, so a second run writes nothing. A merge NEVER
//! completes or blocks the card — kanban is the authority; the comment
//! is evidence for humans.

use std::collections::BTreeSet;

use crate::config::Config;
use crate::github::{CheckRun, GitHub, GitHubError};
use crate::kanban::CardView;
use crate::link::LinkMap;

/// Issue refs (in `owner/repo#n` form) that got an evidence comment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrReport {
    /// Evidence comments posted, in card order.
    pub posted: Vec<String>,
    /// What a dry run would post.
    pub would_post: Vec<String>,
    /// (issue, reason): not merged, already posted.
    pub skipped: Vec<(String, String)>,
}

/// Find the first GitHub pull-request URL in `text`.
///
/// Matches `https://github.com/<owner>/<repo>/pull/<n>` where the owner
/// and repo are identifier-like and `<n>` has at least one digit.
/// Non-pull URLs (issues, repos) do not match.
pub fn extract_pr_url(text: &str) -> Option<(String, String, u32)> {
    const MARKER: &str = "https://github.com/";
    let is_ident = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    };
    let mut pos = 0;
    while let Some(rel) = text[pos..].find(MARKER) {
        let after = &text[pos + rel + MARKER.len()..];
        if let Some((owner, rest)) = after.split_once('/')
            && let Some((repo, rest)) = rest.split_once('/')
            && let Some(tail) = rest.strip_prefix("pull/")
        {
            let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
            if is_ident(owner)
                && is_ident(repo)
                && let Ok(n) = digits.parse::<u32>()
            {
                return Some((owner.to_string(), repo.to_string(), n));
            }
        }
        pos = pos + rel + MARKER.len();
    }
    None
}

/// Collapse check runs into one CI verdict:
/// any failed/cancelled run → `failure`; all runs completed and green →
/// `success`; anything else (in flight, or nothing at all) → `pending`.
/// Failures are surfaced, never hidden.
pub fn ci_rollup(runs: &[CheckRun]) -> &'static str {
    let mut completed = 0usize;
    for r in runs {
        match r.conclusion.as_deref() {
            Some("failure") | Some("cancelled") | Some("timed_out") | Some("action_required") => {
                return "failure";
            }
            Some("success") | Some("neutral") | Some("skipped") => completed += 1,
            _ => {}
        }
    }
    if completed > 0 && completed == runs.len() {
        "success"
    } else {
        "pending"
    }
}

/// The evidence comment text for a merged PR.
pub fn evidence_comment(number: u32, rollup: &str, contract: &str) -> String {
    format!("PR #{number} merged - CI: {rollup} - contract: {contract}")
}

/// Post PR evidence for every linked card that references a PR of this
/// org's repos.
pub async fn post_pr_evidence(
    gh: &mut impl GitHub,
    cfg: &Config,
    views: &[CardView],
    links: &LinkMap,
    dry_run: bool,
) -> Result<PrReport, GitHubError> {
    let mut report = PrReport::default();
    let repos: Vec<String> = cfg.repos.iter().map(|r| r.name.clone()).collect();
    let mut sorted: Vec<&CardView> = views.iter().collect();
    sorted.sort_by(|a, b| a.card.id.cmp(&b.card.id));

    for view in sorted {
        let card = &view.card;
        let Some(linked) = links.issue_for_card(&card.id) else {
            continue;
        };
        // Candidates: PR URLs in the completion contract and the body.
        let mut seen: BTreeSet<(String, u32)> = BTreeSet::new();
        let mut candidates: Vec<(&str, Option<&str>)> = Vec::new();
        if let Some(contract) = card.completion_contract.as_deref() {
            candidates.push((contract, Some(contract)));
        }
        candidates.push((card.body.as_str(), None));
        for (text, _contract) in candidates {
            let Some((owner, repo, number)) = extract_pr_url(text) else {
                continue;
            };
            // Scoped to this org's configured repos; one pass per PR.
            if owner != cfg.org || !repos.contains(&repo) || !seen.insert((repo.clone(), number)) {
                continue;
            }
            let pull = gh.get_pull(&repo, number).await?;
            if !pull.merged {
                report
                    .skipped
                    .push((linked.key(), format!("PR #{number} not merged")));
                continue;
            }
            let runs = gh.check_runs(&repo, &pull.head_sha).await?;
            let rollup = ci_rollup(&runs);
            let contract = card
                .completion_contract
                .clone()
                .unwrap_or_else(|| "none".into());
            let comment = evidence_comment(number, rollup, &contract);
            // Dedupe: an identical comment already on the issue.
            let existing = gh.issue_comments(&linked.repo, linked.number).await?;
            if existing.iter().any(|c| c.body == comment) {
                report.skipped.push((
                    linked.key(),
                    format!("PR #{number} evidence already posted"),
                ));
                continue;
            }
            if dry_run {
                report.would_post.push(linked.key());
                continue;
            }
            gh.comment(&linked.repo, linked.number, &comment).await?;
            report.posted.push(linked.key());
        }
    }
    Ok(report)
}
