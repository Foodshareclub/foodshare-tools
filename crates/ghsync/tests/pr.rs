//! T10 — PR evidence: a merged PR leaves exactly one evidence comment on
//! the linked issue. A merge NEVER completes or blocks the card — kanban
//! is the authority; the comment is evidence for humans.
//!
//! Fakes only; the live run is blocked on the org PAT (T0).

use ghsync::config::{Config, RepoConfig, card_link_comment, issue_link_comment, labels_for};
use ghsync::fakes::{FakeGitHub, FakeKanban};
use ghsync::github::GitHub;
use ghsync::kanban::{CardView, Kanban};
use ghsync::link::resolve_links_views;
use ghsync::pr::{PrReport, ci_rollup, extract_pr_url, post_pr_evidence};

const REPO: &str = "foodshare-tools";

fn test_config() -> Config {
    Config {
        repos: vec![RepoConfig {
            name: REPO.into(),
            default_milestone: "Rust + TypeScript".into(),
            ..Default::default()
        }],
        ..Config::default()
    }
}

/// One linked card (body carries the PR URL) + its issue.
async fn linked_card_with_pr(
    kb: &mut FakeKanban,
    gh: &mut FakeGitHub,
    card_body: &str,
    contract: Option<&str>,
    card_status: &str,
) -> String {
    let card = kb
        .create("Card", card_body, "default", "k1", card_status)
        .await
        .unwrap();
    if let Some(c) = contract {
        kb.cards
            .iter_mut()
            .find(|c| c.id == card.id)
            .unwrap()
            .completion_contract = Some(c.to_string());
    }
    let footer = card_link_comment("foodshare", &card.id);
    gh.create_issue(
        REPO,
        "Card",
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

async fn run_pr(kb: &mut FakeKanban, gh: &mut FakeGitHub, cfg: &Config, dry: bool) -> PrReport {
    let cards = kb.list().await.unwrap();
    let mut vs: Vec<CardView> = Vec::new();
    for c in &cards {
        vs.push(kb.show(&c.id).await.unwrap());
    }
    let links = resolve_links_views(gh, &cfg.board, &cfg.org, &[REPO.to_string()], &vs)
        .await
        .unwrap();
    post_pr_evidence(gh, cfg, &vs, &links, dry).await.unwrap()
}

fn seed_merged_pr(gh: &mut FakeGitHub, sha: &str, runs: Vec<ghsync::github::CheckRun>) {
    gh.pulls.insert(
        (REPO.to_string(), 42),
        ghsync::github::Pull {
            number: 42,
            merged: true,
            head_sha: sha.into(),
            url: format!("https://github.com/Foodshareclub/{REPO}/pull/42"),
        },
    );
    if !runs.is_empty() {
        gh.check_runs.insert(sha.to_string(), runs);
    }
}

fn run(status: &str, conclusion: Option<&str>) -> ghsync::github::CheckRun {
    ghsync::github::CheckRun {
        status: status.into(),
        conclusion: conclusion.map(str::to_string),
    }
}

#[tokio::test]
async fn merged_pr_with_green_rollup_posts_exactly_one_comment() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let body = "Ship it\n\nhttps://github.com/Foodshareclub/foodshare-tools/pull/42";
    linked_card_with_pr(&mut kb, &mut gh, body, Some("local-only"), "ready").await;
    seed_merged_pr(
        &mut gh,
        "sha1",
        vec![
            run("completed", Some("success")),
            run("completed", Some("success")),
        ],
    );
    let cfg = test_config();

    let report = run_pr(&mut kb, &mut gh, &cfg, false).await;

    assert_eq!(
        report.posted,
        vec!["Foodshareclub/foodshare-tools#1".to_string()]
    );
    assert!(report.skipped.is_empty());
    let comments = &gh.comments_posted;
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].0, REPO);
    assert_eq!(comments[0].1, 1);
    assert_eq!(
        comments[0].2,
        "PR #42 merged - CI: success - contract: local-only"
    );
    // The merge never touches the card.
    assert_eq!(kb.cards[0].status, "ready");
    assert!(kb.blocked.is_empty());
    assert!(kb.completed.is_empty());
}

#[tokio::test]
async fn second_run_posts_no_duplicate() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let body = "https://github.com/Foodshareclub/foodshare-tools/pull/42";
    linked_card_with_pr(&mut kb, &mut gh, body, None, "running").await;
    seed_merged_pr(&mut gh, "sha1", vec![run("completed", Some("success"))]);
    let cfg = test_config();

    let first = run_pr(&mut kb, &mut gh, &cfg, false).await;
    assert_eq!(first.posted.len(), 1);
    let second = run_pr(&mut kb, &mut gh, &cfg, false).await;

    assert!(second.posted.is_empty());
    assert_eq!(second.skipped.len(), 1);
    assert!(second.skipped[0].1.contains("already posted"));
    assert_eq!(gh.comments_posted.len(), 1, "still exactly one comment");
}

#[tokio::test]
async fn unmerged_pr_posts_nothing() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let body = "https://github.com/Foodshareclub/foodshare-tools/pull/42";
    linked_card_with_pr(&mut kb, &mut gh, body, None, "running").await;
    gh.pulls.insert(
        (REPO.to_string(), 42),
        ghsync::github::Pull {
            number: 42,
            merged: false,
            head_sha: "sha1".into(),
            url: format!("https://github.com/Foodshareclub/{REPO}/pull/42"),
        },
    );
    let cfg = test_config();

    let report = run_pr(&mut kb, &mut gh, &cfg, false).await;

    assert!(report.posted.is_empty());
    assert_eq!(gh.comments_posted.len(), 0);
    assert_eq!(report.skipped.len(), 1);
    assert!(report.skipped[0].1.contains("not merged"));
}

#[tokio::test]
async fn merged_pr_with_failing_rollup_says_failure() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let body = "https://github.com/Foodshareclub/foodshare-tools/pull/42";
    linked_card_with_pr(&mut kb, &mut gh, body, Some("gh-pr"), "running").await;
    seed_merged_pr(
        &mut gh,
        "sha1",
        vec![
            run("completed", Some("success")),
            run("completed", Some("failure")),
        ],
    );
    let cfg = test_config();

    let report = run_pr(&mut kb, &mut gh, &cfg, false).await;

    assert_eq!(report.posted.len(), 1);
    assert_eq!(
        gh.comments_posted[0].2,
        "PR #42 merged - CI: failure - contract: gh-pr"
    );
}

#[tokio::test]
async fn card_completed_before_merge_still_gets_evidence_once() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let body = "https://github.com/Foodshareclub/foodshare-tools/pull/42";
    // The card is already done; the PR merges afterwards.
    linked_card_with_pr(&mut kb, &mut gh, body, None, "done").await;
    seed_merged_pr(&mut gh, "sha1", vec![run("completed", Some("success"))]);
    let cfg = test_config();

    let report = run_pr(&mut kb, &mut gh, &cfg, false).await;

    assert_eq!(report.posted.len(), 1);
    assert_eq!(gh.comments_posted.len(), 1);
    // And it stays one.
    let again = run_pr(&mut kb, &mut gh, &cfg, false).await;
    assert!(again.posted.is_empty());
    assert_eq!(gh.comments_posted.len(), 1);
}

#[tokio::test]
async fn pr_url_in_contract_and_foreign_orgs_are_scoped() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    // No URL in the body; the contract carries it.
    linked_card_with_pr(
        &mut kb,
        &mut gh,
        "no url here",
        Some("https://github.com/Foodshareclub/foodshare-tools/pull/42"),
        "running",
    )
    .await;
    seed_merged_pr(&mut gh, "sha1", vec![run("completed", Some("success"))]);
    let cfg = test_config();

    let report = run_pr(&mut kb, &mut gh, &cfg, false).await;

    assert_eq!(report.posted.len(), 1, "contract URL is a candidate");
    assert_eq!(
        gh.comments_posted[0].2,
        "PR #42 merged - CI: success - contract: https://github.com/Foodshareclub/foodshare-tools/pull/42"
    );

    // A PR in another org is ignored (evidence only for this org's repos).
    let mut kb2 = FakeKanban::default();
    let mut gh2 = FakeGitHub::default();
    let foreign = "see https://github.com/someone-else/other/pull/9";
    linked_card_with_pr(&mut kb2, &mut gh2, foreign, None, "running").await;
    let report2 = run_pr(&mut kb2, &mut gh2, &cfg, false).await;
    assert!(report2.posted.is_empty());
    assert!(gh2.comments_posted.is_empty());
}

#[tokio::test]
async fn dry_run_reports_without_posting() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let body = "https://github.com/Foodshareclub/foodshare-tools/pull/42";
    linked_card_with_pr(&mut kb, &mut gh, body, None, "running").await;
    seed_merged_pr(&mut gh, "sha1", vec![run("completed", Some("success"))]);
    let cfg = test_config();

    let report = run_pr(&mut kb, &mut gh, &cfg, true).await;

    assert_eq!(
        report.would_post,
        vec!["Foodshareclub/foodshare-tools#1".to_string()]
    );
    assert!(report.posted.is_empty());
    assert!(gh.comments_posted.is_empty());
}

#[test]
fn extract_pr_url_finds_owner_repo_number() {
    assert_eq!(
        extract_pr_url("done in https://github.com/Foodshareclub/foodshare-app/pull/7."),
        Some(("Foodshareclub".into(), "foodshare-app".into(), 7))
    );
    assert_eq!(
        extract_pr_url("https://github.com/Foodshareclub/foodshare-tools/pull/42"),
        Some(("Foodshareclub".into(), "foodshare-tools".into(), 42))
    );
    // Not a pull URL.
    assert_eq!(
        extract_pr_url("https://github.com/Foodshareclub/foodshare-tools/issues/42"),
        None
    );
    // No digits after /pull/.
    assert_eq!(extract_pr_url("https://github.com/a/b/pull/"), None);
    // No URL at all.
    assert_eq!(extract_pr_url("plain text"), None);
    // The first valid URL wins when several appear.
    assert_eq!(
        extract_pr_url(
            "https://github.com/x/y/pull/1 and https://github.com/Foodshareclub/foodshare-tools/pull/2"
        ),
        Some(("x".into(), "y".into(), 1))
    );
}

#[test]
fn ci_rollup_aggregates_check_runs() {
    // All green.
    assert_eq!(
        ci_rollup(&[
            run("completed", Some("success")),
            run("completed", Some("neutral")),
        ]),
        "success"
    );
    // One failure wins.
    assert_eq!(
        ci_rollup(&[
            run("completed", Some("success")),
            run("completed", Some("failure")),
        ]),
        "failure"
    );
    // Cancelled counts as failure.
    assert_eq!(ci_rollup(&[run("completed", Some("cancelled"))]), "failure");
    // Incomplete: pending.
    assert_eq!(
        ci_rollup(&[run("completed", Some("success")), run("in_progress", None),]),
        "pending"
    );
    // No check runs at all: pending (nothing to fail, nothing green).
    assert_eq!(ci_rollup(&[]), "pending");
}
