//! T11 — discussion feed + discussion intake.
//!
//! Feed (kanban → discussions): a card with a `sync:github <verdict>`
//! comment or an `escalation:` block gets one discussion in the repo's
//! Handoffs category; a later run appends to it, never creates a second.
//! Intake (discussions → kanban): a discussion labeled `intake` becomes
//! one card.
//!
//! Fakes only; the live run is blocked on the org PAT (T0).

use ghsync::config::{
    Config, RepoConfig, discussion_link_comment, issue_link_comment, labels_for, parse_issue_link,
};
use ghsync::fakes::{FakeGitHub, FakeKanban};
use ghsync::github::{Discussion, GitHub};
use ghsync::kanban::{CardView, Kanban};
use ghsync::link::resolve_links_views;

const REPO: &str = "foodshare-tools";
const BOARD: &str = "foodshare";

fn test_config() -> Config {
    Config {
        repos: vec![RepoConfig {
            name: REPO.into(),
            default_milestone: "Rust + TypeScript".into(),
            discussions_enabled: true,
            discussion_labels_enabled: true,
        }],
        ..Config::default()
    }
}

/// One linked card (+ issue) the way push left it.
async fn linked_card(kb: &mut FakeKanban, gh: &mut FakeGitHub, title: &str) -> String {
    let card = kb
        .create(title, "x", "default", "k1", "review")
        .await
        .unwrap();
    let footer = ghsync::config::card_link_comment(BOARD, &card.id);
    gh.create_issue(
        REPO,
        title,
        &format!("body\n\n---\n{footer}"),
        &labels_for(REPO),
    )
    .await
    .unwrap();
    kb.comment(&card.id, &issue_link_comment("Foodshareclub", REPO, 1))
        .await
        .unwrap();
    card.id
}

async fn views(kb: &mut FakeKanban) -> Vec<CardView> {
    let cards = kb.list().await.unwrap();
    let mut out = Vec::new();
    for c in &cards {
        out.push(kb.show(&c.id).await.unwrap());
    }
    out
}

async fn run_feed(
    kb: &mut FakeKanban,
    gh: &mut FakeGitHub,
    cfg: &Config,
) -> ghsync::discuss::FeedReport {
    let vs = views(kb).await;
    let links = resolve_links_views(gh, &cfg.board, &cfg.org, &[REPO.to_string()], &vs)
        .await
        .unwrap();
    ghsync::discuss::post_feed(gh, cfg, &vs, &links)
        .await
        .unwrap()
}

fn seed_discussion(gh: &mut FakeGitHub, number: u32, title: &str, body: &str, labels: &[&str]) {
    let d = Discussion {
        id: format!("D{number}"),
        number,
        title: title.into(),
        body: body.into(),
        labels: labels
            .iter()
            .map(|n| ghsync::github::DiscussionLabel {
                id: format!("L{number}_{n}"),
                name: (*n).to_string(),
            })
            .collect(),
        comments: Vec::new(),
    };
    gh.discussions.entry(REPO.to_string()).or_default().push(d);
}

#[tokio::test]
async fn approval_verdict_creates_one_discussion_second_run_appends_at_most_one_comment() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let card_id = linked_card(&mut kb, &mut gh, "Fix the flaky test").await;
    kb.comment(&card_id, "sync:github approve").await.unwrap();
    let cfg = test_config();

    let first = run_feed(&mut kb, &mut gh, &cfg).await;
    assert_eq!(first.created, 1);
    assert_eq!(first.appended, 0);
    assert_eq!(gh.created_discussions.len(), 1);
    let (repo, category, title) = &gh.created_discussions[0];
    assert_eq!(repo, REPO);
    assert_eq!(category, "Handoffs");
    assert_eq!(title, "[approve] Fix the flaky test");
    // The body re-derives the card link.
    let d = &gh.discussions[REPO][0];
    assert!(d.body.contains(&format!("Card: kanban:{BOARD}:{card_id}")));

    // Second push: no second discussion; at most one appended comment.
    let second = run_feed(&mut kb, &mut gh, &cfg).await;
    assert_eq!(second.created, 0);
    assert_eq!(gh.created_discussions.len(), 1, "never a second discussion");
    assert!(
        second.appended <= 1,
        "at most one appended comment: {second:?}"
    );
}

#[tokio::test]
async fn escalation_block_creates_discussion_titled_escalation() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let card_id = linked_card(&mut kb, &mut gh, "Needs a decision").await;
    kb.block(
        &card_id,
        "needs_input",
        "escalation: API key for staging missing",
    )
    .await
    .unwrap();
    let cfg = test_config();

    let report = run_feed(&mut kb, &mut gh, &cfg).await;

    assert_eq!(report.created, 1);
    let title = &gh.created_discussions[0].2;
    assert!(
        title.contains("escalation"),
        "title must name the escalation: {title}"
    );
    assert_eq!(title, "[escalation] Needs a decision");
}

#[tokio::test]
async fn card_without_verdict_gets_no_discussion() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    linked_card(&mut kb, &mut gh, "Just work").await;
    let cfg = test_config();

    let report = run_feed(&mut kb, &mut gh, &cfg).await;

    assert_eq!(report.created, 0);
    assert!(gh.created_discussions.is_empty());
}

#[tokio::test]
async fn repo_without_discussions_is_skipped_with_zero_calls() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let card_id = linked_card(&mut kb, &mut gh, "Fix it").await;
    kb.comment(&card_id, "sync:github approve").await.unwrap();
    let cfg = Config {
        repos: vec![RepoConfig {
            name: REPO.into(),
            default_milestone: "Rust + TypeScript".into(),
            discussions_enabled: false,
            discussion_labels_enabled: false,
        }],
        ..Config::default()
    };

    let report = run_feed(&mut kb, &mut gh, &cfg).await;

    assert_eq!(report.created, 0);
    assert_eq!(report.skipped.len(), 1);
    assert!(
        report.skipped[0].1.contains("discussions disabled"),
        "skip must be logged: {report:?}"
    );
    assert!(gh.created_discussions.is_empty());
    assert!(gh.discussion_comments.is_empty());

    // Intake side too: with a discussion labeled intake but the repo
    // gated off, nothing is created.
    seed_discussion(&mut gh, 7, "Some work", "body", &["intake"]);
    let report2 = ghsync::discuss::intake_discussions(&mut kb, &mut gh, &cfg, false)
        .await
        .unwrap();
    assert!(report2.created.is_empty());
    assert!(
        !kb.created
            .iter()
            .any(|c| c.idempotency_key.starts_with("ghd-")),
        "no discussion card may be created for a gated repo: {:?}",
        kb.created
    );
    assert!(gh.removed_labels.is_empty());
}

#[tokio::test]
async fn discussion_intake_creates_one_card_with_key_and_removes_label() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    seed_discussion(
        &mut gh,
        7,
        "Please add a release note",
        "We need a release note for 1.4.",
        &["intake"],
    );
    let cfg = test_config();

    let report = ghsync::discuss::intake_discussions(&mut kb, &mut gh, &cfg, false)
        .await
        .unwrap();

    assert_eq!(report.created.len(), 1);
    assert_eq!(report.created[0].issue, "Foodshareclub/foodshare-tools#7");
    assert_eq!(kb.created.len(), 1);
    let card = &kb.created[0];
    assert_eq!(card.idempotency_key, "ghd-Foodshareclub-foodshare-tools-7");
    assert_eq!(
        card.title,
        "Please add a release note (ghd Foodshareclub/foodshare-tools#7)"
    );
    // Link comment on the card uses the discussion prefix.
    let comments = &kb.comments[&card.id];
    assert_eq!(
        comments,
        &vec![discussion_link_comment("Foodshareclub", REPO, 7)]
    );
    // The intake label is removed from the discussion.
    assert_eq!(gh.removed_labels.len(), 1);
    assert_eq!(gh.removed_labels[0].2, "L7_intake");
    let d = &gh.discussions[REPO][0];
    assert!(d.labels.is_empty());

    // Second run: the discussion no longer carries the label -> no-op.
    let second = ghsync::discuss::intake_discussions(&mut kb, &mut gh, &cfg, false)
        .await
        .unwrap();
    assert!(second.created.is_empty());
    assert_eq!(kb.created.len(), 1, "still exactly one card");
}

#[tokio::test]
async fn discussion_intake_requires_label_support() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    seed_discussion(&mut gh, 7, "Please add a release note", "body", &["intake"]);
    let cfg = Config {
        repos: vec![RepoConfig {
            name: REPO.into(),
            default_milestone: "Rust + TypeScript".into(),
            discussions_enabled: true,
            discussion_labels_enabled: false,
        }],
        ..Config::default()
    };

    let report = ghsync::discuss::intake_discussions(&mut kb, &mut gh, &cfg, false)
        .await
        .unwrap();

    assert!(report.created.is_empty());
    assert_eq!(report.skipped.len(), 1);
    assert!(
        report.skipped[0].1.contains("labels unsupported"),
        "skip must be logged: {report:?}"
    );
    assert!(kb.created.is_empty());
}

#[test]
fn discussion_link_comment_round_trip_parses_as_issue_link() {
    let line = discussion_link_comment("Foodshareclub", REPO, 7);
    assert_eq!(line, "ghsyncd Foodshareclub/foodshare-tools#7");
    let parsed = parse_issue_link(&line).expect("ghsyncd line parses");
    assert_eq!(parsed.owner, "Foodshareclub");
    assert_eq!(parsed.repo, REPO);
    assert_eq!(parsed.number, 7);
    // The plain issue prefix still parses, and a ghsyncd line is NOT
    // misread as a ghsync line (and vice versa).
    let plain = issue_link_comment("Foodshareclub", REPO, 7);
    assert_eq!(plain, "ghsync Foodshareclub/foodshare-tools#7");
    assert!(parse_issue_link(&plain).is_some());
    assert!(parse_issue_link("ghsyncd something-else").is_none());
}

#[test]
fn unknown_verdict_token_is_not_a_verdict() {
    // `sync:github whatever` with an unknown token is logged, not posted.
    assert_eq!(
        ghsync::discuss::verdict_from_line("sync:github probably"),
        None
    );
    assert_eq!(
        ghsync::discuss::verdict_from_line("sync:github approve"),
        Some("approve")
    );
    assert_eq!(
        ghsync::discuss::verdict_from_line("sync:github request-changes"),
        Some("request-changes")
    );
}
