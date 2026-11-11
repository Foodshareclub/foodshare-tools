//! T5 — push part 1: link-or-create issues, milestones, state gestures.
//!
//! Kanban is the single source of state: push derives everything from the
//! board and only writes the GitHub side. All tests run against the
//! in-memory fakes — no network, no live GitHub.

use ghsync::config::{Config, RepoConfig, card_link_comment, issue_link_comment, labels_for};
use ghsync::fakes::{FakeGitHub, FakeKanban};
use ghsync::github::GitHub;
use ghsync::kanban::{CardView, Kanban};
use ghsync::link::resolve_links_views;
use ghsync::push::{FOOTER_NOTE, PushReport, push_issues};

/// The exact reason push blocks a card when a human closes its issue.
const CLOSED_REASON: &str = "issue closed on GitHub (ghsync)";
/// The exact reason push unblocks a card when the issue is reopened.
const REOPEN_REASON: &str = "issue reopened on GitHub (ghsync)";

fn test_config() -> Config {
    Config {
        repos: vec![
            RepoConfig {
                name: "foodshare-app".into(),
                default_milestone: "Next.js 16 + Bun 1.4.4".into(),
            },
            RepoConfig {
                name: "foodshare-tools".into(),
                default_milestone: "Rust + TypeScript".into(),
            },
        ],
        ..Config::default()
    }
}

async fn card_views(kb: &mut FakeKanban) -> Vec<CardView> {
    let cards = kb.list().await.unwrap();
    let mut views = Vec::new();
    for c in &cards {
        views.push(kb.show(&c.id).await.unwrap());
    }
    views
}

async fn run(kb: &mut FakeKanban, gh: &mut FakeGitHub, cfg: &Config, dry: bool) -> PushReport {
    let views = card_views(kb).await;
    let repos: Vec<String> = cfg.repos.iter().map(|r| r.name.clone()).collect();
    let links = resolve_links_views(gh, &cfg.board, &cfg.org, &repos, &views)
        .await
        .unwrap();
    push_issues(kb, gh, cfg, &views, &links, dry).await.unwrap()
}

/// Pre-link a card to a brand-new issue the way a previous push would have
/// left it: footer in the issue body, link comment on the card.
async fn prelink(kb: &mut FakeKanban, gh: &mut FakeGitHub, card_id: &str, repo: &str) -> u32 {
    let footer = card_link_comment("foodshare", card_id);
    let issue = gh
        .create_issue(
            repo,
            "Linked card",
            &format!("body\n\n---\n{footer}"),
            &labels_for(repo),
        )
        .await
        .unwrap();
    kb.comment(
        card_id,
        &issue_link_comment("Foodshareclub", repo, issue.number),
    )
    .await
    .unwrap();
    issue.number
}

#[tokio::test]
async fn unlinked_card_creates_issue_with_footer_labels_milestone_and_link_comment() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let cfg = test_config();
    let card = kb
        .create("New card", "do the thing", "default", "k1", "ready")
        .await
        .unwrap();

    let report = run(&mut kb, &mut gh, &cfg, false).await;

    assert_eq!(report.created, vec![card.id.clone()]);
    assert!(report.skipped.is_empty(), "skipped: {:?}", report.skipped);
    let issue = &gh.issues[&("foodshare-tools".to_string(), 1)];
    assert_eq!(issue.title, "New card");
    // Body is the card body plus the link footer.
    assert_eq!(
        issue.body,
        format!(
            "do the thing\n\n---\n{}",
            card_link_comment(&cfg.board, &card.id)
        )
    );
    // Labels are the ghsync set for the repo.
    assert_eq!(issue.labels, labels_for("foodshare-tools"));
    // Milestone is the repo's default, found-or-created.
    assert_eq!(issue.milestone.as_deref(), Some("Rust + TypeScript"));
    assert_eq!(
        gh.milestone_set
            .get(&("foodshare-tools".to_string(), 1))
            .copied(),
        Some(1)
    );
    // The card carries the stateless link comment.
    let v = kb.show(&card.id).await.unwrap();
    assert_eq!(
        v.comments,
        vec![issue_link_comment(&cfg.org, "foodshare-tools", 1)]
    );
}

#[tokio::test]
async fn second_push_is_idempotent() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let cfg = test_config();
    kb.create("New card", "do the thing", "default", "k1", "ready")
        .await
        .unwrap();

    let first = run(&mut kb, &mut gh, &cfg, false).await;
    assert_eq!(first.created.len(), 1);

    let comments_before = gh.comments_posted.len();
    let bodies_before = gh.bodies_set.len();
    let closes_before = gh.closed_calls.len();
    let report = run(&mut kb, &mut gh, &cfg, false).await;

    assert!(report.created.is_empty());
    assert!(report.would_create.is_empty());
    assert!(report.gestures.is_empty());
    assert_eq!(gh.issues.len(), 1);
    assert_eq!(gh.milestones["foodshare-tools"].len(), 1);
    assert_eq!(gh.comments_posted.len(), comments_before);
    assert_eq!(gh.bodies_set.len(), bodies_before);
    assert_eq!(gh.closed_calls.len(), closes_before);
}

#[tokio::test]
async fn dry_run_writes_nothing() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let cfg = test_config();
    let card = kb
        .create("New card", "do the thing", "default", "k1", "ready")
        .await
        .unwrap();

    let report = run(&mut kb, &mut gh, &cfg, true).await;

    assert!(report.created.is_empty());
    assert_eq!(report.would_create, vec![card.id.clone()]);
    assert!(gh.issues.is_empty());
    assert!(gh.milestones.is_empty());
    assert!(gh.closed_calls.is_empty());
    assert!(gh.comments_posted.is_empty());
    let v = kb.show(&card.id).await.unwrap();
    assert!(v.comments.is_empty());
}

#[tokio::test]
async fn done_card_closes_open_issue_exactly_once() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let cfg = test_config();
    let card = kb
        .create("Ship it", "x", "default", "k1", "done")
        .await
        .unwrap();
    let n = prelink(&mut kb, &mut gh, &card.id, "foodshare-tools").await;

    let r1 = run(&mut kb, &mut gh, &cfg, false).await;

    assert_eq!(
        gh.issues[&("foodshare-tools".to_string(), n)].state,
        "closed"
    );
    assert_eq!(gh.closed_calls, vec![("foodshare-tools".to_string(), n)]);
    assert!(
        r1.gestures
            .iter()
            .any(|g| g.card == card.id && g.action == "close"),
        "gestures: {:?}",
        r1.gestures
    );

    // Second push: the issue is already closed — no additional close.
    let r2 = run(&mut kb, &mut gh, &cfg, false).await;
    assert_eq!(gh.closed_calls.len(), 1);
    assert!(r2.gestures.is_empty());
}

#[tokio::test]
async fn human_closed_issue_blocks_card_and_reopen_unblocks() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let cfg = test_config();
    let card = kb
        .create("WIP", "x", "default", "k1", "running")
        .await
        .unwrap();
    let n = prelink(&mut kb, &mut gh, &card.id, "foodshare-tools").await;
    // A human closes the issue on GitHub.
    gh.close_issue("foodshare-tools", n).await.unwrap();

    let r1 = run(&mut kb, &mut gh, &cfg, false).await;

    assert_eq!(
        kb.blocked,
        vec![(card.id.clone(), "dependency".into(), CLOSED_REASON.into())]
    );
    assert!(
        r1.drift
            .iter()
            .any(|d| d.card == card.id && d.kind == "issue-closed-by-human"),
        "drift: {:?}",
        r1.drift
    );
    // The milestone is still enforced on the linked issue.
    assert_eq!(
        gh.issues[&("foodshare-tools".to_string(), n)]
            .milestone
            .as_deref(),
        Some("Rust + TypeScript")
    );

    // Second push: already blocked for the right reason — no second block.
    run(&mut kb, &mut gh, &cfg, false).await;
    assert_eq!(kb.blocked.len(), 1);

    // A human reopens the issue.
    gh.issues
        .get_mut(&("foodshare-tools".to_string(), n))
        .unwrap()
        .state = "open".into();

    let r3 = run(&mut kb, &mut gh, &cfg, false).await;

    assert_eq!(kb.unblocked, vec![(card.id.clone(), REOPEN_REASON.into())]);
    assert!(
        r3.gestures
            .iter()
            .any(|g| g.card == card.id && g.action == "unblock"),
        "gestures: {:?}",
        r3.gestures
    );
    let v = kb.show(&card.id).await.unwrap();
    assert_eq!(v.card.status, "ready");
}

#[tokio::test]
async fn unknown_repo_is_drift_not_error() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let cfg = test_config();
    let card = kb
        .create(
            "Odd",
            "Repo: not-a-configured-repo\nx",
            "default",
            "k1",
            "ready",
        )
        .await
        .unwrap();

    let report = run(&mut kb, &mut gh, &cfg, false).await;

    assert!(
        report
            .drift
            .iter()
            .any(|d| d.card == card.id && d.kind == "unknown-repo"),
        "drift: {:?}",
        report.drift
    );
    assert!(
        report
            .skipped
            .iter()
            .any(|(id, why)| id == &card.id && why == "unknown-repo"),
        "skipped: {:?}",
        report.skipped
    );
    assert!(gh.issues.is_empty());
    assert!(report.created.is_empty());
}

#[tokio::test]
async fn stripped_footer_is_restored_in_body_once() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let cfg = test_config();
    let card = kb.create("F", "x", "default", "k1", "ready").await.unwrap();
    let n = prelink(&mut kb, &mut gh, &card.id, "foodshare-tools").await;
    // A human strips the footer from the issue body.
    gh.issues
        .get_mut(&("foodshare-tools".to_string(), n))
        .unwrap()
        .body = "stripped".into();

    let r1 = run(&mut kb, &mut gh, &cfg, false).await;

    assert!(
        r1.drift
            .iter()
            .any(|d| d.card == card.id && d.kind == "footer-restored"),
        "drift: {:?}",
        r1.drift
    );
    let footer = card_link_comment(&cfg.board, &card.id);
    assert_eq!(
        gh.bodies_set.get(&("foodshare-tools".to_string(), n)),
        Some(&format!("stripped\n\n---\n{footer}"))
    );
    assert!(
        gh.comments_posted
            .iter()
            .any(|(_, num, body)| *num == n && body == FOOTER_NOTE),
        "comments: {:?}",
        gh.comments_posted
    );

    // Second push: the body carries the footer now — nothing more is written.
    let comments_before = gh.comments_posted.len();
    let r2 = run(&mut kb, &mut gh, &cfg, false).await;
    assert_eq!(gh.comments_posted.len(), comments_before);
    assert!(
        !r2.drift.iter().any(|d| d.kind == "footer-restored"),
        "drift: {:?}",
        r2.drift
    );
}

#[tokio::test]
async fn existing_milestone_is_reused_not_duplicated() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let cfg = test_config();
    // The milestone already exists in the repo.
    gh.create_milestone("foodshare-tools", "Rust + TypeScript")
        .await
        .unwrap();
    let card = kb.create("F", "x", "default", "k1", "ready").await.unwrap();
    let n = prelink(&mut kb, &mut gh, &card.id, "foodshare-tools").await;

    let report = run(&mut kb, &mut gh, &cfg, false).await;

    assert!(report.created.is_empty());
    assert_eq!(gh.milestones["foodshare-tools"].len(), 1);
    assert_eq!(
        gh.issues[&("foodshare-tools".to_string(), n)]
            .milestone
            .as_deref(),
        Some("Rust + TypeScript")
    );
    assert_eq!(
        gh.milestone_set
            .get(&("foodshare-tools".to_string(), n))
            .copied(),
        Some(1)
    );
}
