//! T4 — stateless link resolution: the map is re-derived from the two
//! sides on every run (card comments + issue footers), no state DB.

use std::collections::BTreeMap;

use ghsync::config::LinkKind;
use ghsync::fakes::{FakeGitHub, FakeKanban};
use ghsync::github::GitHub;
use ghsync::kanban::{Kanban, LinkedIssue};
use ghsync::link::{LinkMap, card_repo, resolve_links};

async fn seed_card(kb: &mut FakeKanban, title: &str, link: Option<&str>) -> String {
    let card = kb
        .create(title, "body", "default", &format!("key-{title}"), "ready")
        .await
        .unwrap();
    if let Some(link) = link {
        kb.comment(&card.id, link).await.unwrap();
    }
    card.id
}

fn issue(org: &str, repo: &str, number: u32) -> LinkedIssue {
    LinkedIssue {
        owner: org.into(),
        repo: repo.into(),
        number,
        kind: LinkKind::Issue,
    }
}

#[tokio::test]
async fn resolution_builds_bidirectional_map() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    // The issue exists first (labeled ghsync), then the card links to it
    // and the issue carries the footer back to the card.
    let iss = gh
        .create_issue("foodshare-app", "A", "b", &["ghsync".into()])
        .await
        .unwrap();
    let id = seed_card(
        &mut kb,
        "A",
        Some(&format!(
            "ghsync Foodshareclub/foodshare-app#{}",
            iss.number
        )),
    )
    .await;
    gh.comment(
        "foodshare-app",
        iss.number,
        &format!("kanban:foodshare:{id}"),
    )
    .await
    .unwrap();

    let cards = kb.list().await.unwrap();
    let map = resolve_links(
        &mut kb,
        &mut gh,
        "foodshare",
        "Foodshareclub",
        &["foodshare-app".to_string()],
        &cards,
    )
    .await
    .unwrap();

    let got = map.by_card.get(&id).cloned().unwrap_or_default();
    assert_eq!(
        got,
        vec![issue("Foodshareclub", "foodshare-app", iss.number)]
    );
    let back = map
        .by_issue
        .get(&issue("Foodshareclub", "foodshare-app", iss.number))
        .cloned()
        .unwrap_or_default();
    assert_eq!(back, vec![id.clone()]);
    assert!(map.conflict_issues().is_empty());
}

#[tokio::test]
async fn unrelated_card_has_no_links() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    seed_card(&mut kb, "A", None).await;

    let cards = kb.list().await.unwrap();
    let map = resolve_links(
        &mut kb,
        &mut gh,
        "foodshare",
        "Foodshareclub",
        &["foodshare-app".to_string()],
        &cards,
    )
    .await
    .unwrap();
    assert!(map.by_card.values().all(|v| v.is_empty()));
    assert!(map.by_issue.is_empty());
}

#[tokio::test]
async fn two_cards_one_issue_is_a_conflict() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let a = seed_card(&mut kb, "A", Some("ghsync Foodshareclub/foodshare-app#7")).await;
    let b = seed_card(&mut kb, "B", Some("ghsync Foodshareclub/foodshare-app#7")).await;
    gh.create_issue("foodshare-app", "X", "b", &["ghsync".into()])
        .await
        .unwrap();

    let cards = kb.list().await.unwrap();
    let map = resolve_links(
        &mut kb,
        &mut gh,
        "foodshare",
        "Foodshareclub",
        &["foodshare-app".to_string()],
        &cards,
    )
    .await
    .unwrap();

    let conflicts = map.conflict_issues();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].0, issue("Foodshareclub", "foodshare-app", 7));
    let mut ids = conflicts[0].1.clone();
    ids.sort();
    assert_eq!(ids, vec![a, b]);
}

#[tokio::test]
async fn footer_to_missing_card_is_ignored() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    seed_card(&mut kb, "A", None).await;
    let iss = gh
        .create_issue("foodshare-app", "X", "b", &["ghsync".into()])
        .await
        .unwrap();
    gh.comment("foodshare-app", iss.number, "kanban:foodshare:t_gone")
        .await
        .unwrap();

    let cards = kb.list().await.unwrap();
    let map = resolve_links(
        &mut kb,
        &mut gh,
        "foodshare",
        "Foodshareclub",
        &["foodshare-app".to_string()],
        &cards,
    )
    .await
    .unwrap();
    // The footer points at a card that does not exist: the issue is
    // recorded as unlinked, not attached to anything.
    let key = issue("Foodshareclub", "foodshare-app", iss.number);
    assert!(!map.by_issue.contains_key(&key));
    assert!(map.conflict_issues().is_empty());
}

#[tokio::test]
async fn footer_on_other_board_is_ignored() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let id = seed_card(&mut kb, "A", None).await;
    let iss = gh
        .create_issue("foodshare-app", "X", "b", &["ghsync".into()])
        .await
        .unwrap();
    gh.comment(
        "foodshare-app",
        iss.number,
        &format!("kanban:other-board:{id}"),
    )
    .await
    .unwrap();

    let cards = kb.list().await.unwrap();
    let map = resolve_links(
        &mut kb,
        &mut gh,
        "foodshare",
        "Foodshareclub",
        &["foodshare-app".to_string()],
        &cards,
    )
    .await
    .unwrap();
    let key = issue("Foodshareclub", "foodshare-app", iss.number);
    assert!(!map.by_issue.contains_key(&key));
}

// ---------- card repo routing ----------

#[test]
fn repo_line_routes_to_configured_repo() {
    let repos = vec!["foodshare-app".to_string(), "foodshare-tools".to_string()];
    let card = ghsync::kanban::Card {
        body: "some text\nRepo: foodshare-app\nmore".into(),
        ..Default::default()
    };
    assert_eq!(card_repo(&card, &repos), Some("foodshare-app"));
}

#[test]
fn no_repo_line_defaults_to_foodshare_tools() {
    let repos = vec!["foodshare-app".to_string(), "foodshare-tools".to_string()];
    let card = ghsync::kanban::Card {
        body: "no routing here".into(),
        ..Default::default()
    };
    assert_eq!(card_repo(&card, &repos), Some("foodshare-tools"));
}

#[test]
fn unknown_repo_is_none() {
    let repos = vec!["foodshare-app".to_string()];
    let card = ghsync::kanban::Card {
        body: "Repo: catch-22".into(),
        ..Default::default()
    };
    assert_eq!(card_repo(&card, &repos), None);
}

#[test]
fn link_map_helpers_are_consistent() {
    let mut by_card: BTreeMap<String, Vec<LinkedIssue>> = BTreeMap::new();
    let mut by_issue: BTreeMap<LinkedIssue, Vec<String>> = BTreeMap::new();
    let i = issue("Foodshareclub", "foodshare-app", 1);
    by_card.insert("t_1".into(), vec![i.clone()]);
    by_issue.insert(i, vec!["t_1".into()]);
    let map = LinkMap { by_card, by_issue };
    assert_eq!(map.issue_for_card("t_1").map(|x| x.number), Some(1));
    assert!(map.issue_for_card("t_2").is_none());
    assert!(map.conflict_issues().is_empty());
}
