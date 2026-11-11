//! Stateless link resolution (T4) — the no-state-DB core.
//!
//! The card↔issue map is re-derived on every run from the two sides:
//! each card's `ghsync owner/repo#n` comments, and each `ghsync`-labeled
//! issue's `kanban:<board>:<task-id>` footer. Either side can rebuild the
//! map from the other; nothing is stored.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;

use crate::config::{GHSYNC_LABEL, parse_card_link};
use crate::github::GitHub;
use crate::kanban::{Card, Kanban, LinkedIssue};

/// The card↔issue map, in both directions.
#[derive(Debug, Clone, Default)]
pub struct LinkMap {
    /// task_id -> the issues the card is linked to.
    pub by_card: BTreeMap<String, Vec<LinkedIssue>>,
    /// issue -> the cards linked to it.
    pub by_issue: BTreeMap<LinkedIssue, Vec<String>>,
}

impl LinkMap {
    /// Issues linked from more than one card (a system invariant violation;
    /// reported as drift, never silently resolved).
    pub fn conflict_issues(&self) -> Vec<(LinkedIssue, Vec<String>)> {
        self.by_issue
            .iter()
            .filter(|(_, cards)| cards.len() > 1)
            .map(|(issue, cards)| (issue.clone(), cards.clone()))
            .collect()
    }

    /// The issue a card is linked to (the first one, if any).
    pub fn issue_for_card(&self, card_id: &str) -> Option<&LinkedIssue> {
        self.by_card.get(card_id).and_then(|v| v.first())
    }

    /// Every issue this map knows about, in a stable order.
    pub fn all_issues(&self) -> Vec<LinkedIssue> {
        self.by_issue.keys().cloned().collect()
    }
}

/// The repo a card's issue lives in: a `Repo: <name>` line in the card
/// body if present, else `foodshare-tools`. `None` when the named repo is
/// not one of the configured repos (drift, not an error here).
pub fn card_repo<'a>(card: &Card, repos: &'a [String]) -> Option<&'a str> {
    let name = card
        .body
        .lines()
        .find_map(|l| l.trim().strip_prefix("Repo: "))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("foodshare-tools");
    repos
        .iter()
        .find(|r| r.as_str() == name)
        .map(String::as_str)
}

fn push_unique(slot: &mut Vec<String>, value: String) {
    if !slot.contains(&value) {
        slot.push(value);
    }
}

fn push_unique_issue(slot: &mut Vec<LinkedIssue>, value: LinkedIssue) {
    if !slot.contains(&value) {
        slot.push(value);
    }
}

/// Re-derive the link map from both sides.
///
/// Card side: each card's comments are scanned for `ghsync owner/repo#n`
/// lines. Issue side: every `ghsync`-labeled issue in every configured
/// repo is scanned for `kanban:<board>:<task-id>` footers; footers
/// pointing at cards that do not exist on this board are ignored (stale).
pub async fn resolve_links(
    kb: &mut impl Kanban,
    gh: &mut impl GitHub,
    board: &str,
    org: &str,
    repos: &[String],
    cards: &[Card],
) -> Result<LinkMap> {
    let mut by_card: BTreeMap<String, Vec<LinkedIssue>> = BTreeMap::new();
    let mut by_issue: BTreeMap<LinkedIssue, Vec<String>> = BTreeMap::new();

    // Card side: the card's own link comments.
    for card in cards {
        let view = kb.show(&card.id).await?;
        for link in &view.card.links {
            push_unique_issue(by_card.entry(card.id.clone()).or_default(), link.clone());
            push_unique(by_issue.entry(link.clone()).or_default(), card.id.clone());
        }
    }

    // Issue side: the footers on ghsync-labeled issues.
    let card_ids: BTreeSet<&str> = cards.iter().map(|c| c.id.as_str()).collect();
    for repo in repos {
        let issues = gh.issues_labeled(repo, GHSYNC_LABEL).await?;
        for iss in issues {
            let key = LinkedIssue {
                owner: org.to_string(),
                repo: repo.clone(),
                number: iss.number,
            };
            let comments = gh.issue_comments(repo, iss.number).await?;
            for comment in &comments {
                let Some((b, task_id)) = parse_card_link(&comment.body) else {
                    continue;
                };
                if b != board || !card_ids.contains(task_id.as_str()) {
                    continue;
                }
                push_unique(by_issue.entry(key.clone()).or_default(), task_id.clone());
                push_unique_issue(by_card.entry(task_id).or_default(), key.clone());
            }
        }
    }

    Ok(LinkMap { by_card, by_issue })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(body: &str) -> Card {
        Card {
            body: body.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn repo_line_wins_over_default() {
        let repos = vec!["foodshare-app".into(), "foodshare-tools".into()];
        assert_eq!(
            card_repo(&card("Repo: foodshare-app\nx"), &repos),
            Some("foodshare-app")
        );
        assert_eq!(
            card_repo(&card("plain body"), &repos),
            Some("foodshare-tools")
        );
        assert_eq!(card_repo(&card("Repo: unknown-repo"), &repos), None);
        // A `Repo:` mention that is not at the start of a line does not count.
        assert_eq!(
            card_repo(&card("x Repo: foodshare-app"), &repos),
            Some("foodshare-tools")
        );
    }
}
