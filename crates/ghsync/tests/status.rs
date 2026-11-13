//! T12 — `status`: read-only drift table across every surface.
//!
//! One row per linked pair + unlinked cards + orphan issues. Each drift
//! kind must produce exactly its flag; a clean pair reports `ok`.
//! Fakes only; the live run is blocked on the org PAT (T0).

use ghsync::config::{Config, RepoConfig, card_link_comment, labels_for};
use ghsync::fakes::{FakeGitHub, FakeKanban};
use ghsync::github::{GitHub, Pull};
use ghsync::kanban::Kanban;
use ghsync::push::Cursor;
use ghsync::status::run_status;

const REPO: &str = "foodshare-tools";
const BOARD: &str = "foodshare";
const ISSUE_KEY: &str = "Foodshareclub/foodshare-tools#1";

fn test_config() -> Config {
    let mut cfg = Config {
        repos: vec![RepoConfig {
            name: REPO.into(),
            default_milestone: "Rust + TypeScript".into(),
            discussions_enabled: true,
            discussion_labels_enabled: true,
            ..Default::default()
        }],
        ..Config::default()
    };
    if let Some(p) = cfg.project.as_mut() {
        p.project_id = "PV1_fake".into();
        p.status_option_ids = [
            ("Backlog", "opt_backlog"),
            ("Blocked", "opt_blocked"),
            ("Done", "opt_done"),
            ("In progress", "opt_inprog"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        p.phase_option_ids = [
            ("Backlog", "ph_backlog"),
            ("Done", "ph_done"),
            ("In progress", "ph_inprog"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    }
    cfg
}

/// A fully clean linked pair: card `running`, open issue with footer and
/// labels, project item whose fields equal the cursor's last writes.
async fn clean_pair(kb: &mut FakeKanban, gh: &mut FakeGitHub) -> (String, Cursor) {
    let card = kb
        .create("Fix thing", "body", "default", "k1", "running")
        .await
        .unwrap();
    let footer = card_link_comment(BOARD, &card.id);
    gh.create_issue(
        REPO,
        "Fix thing",
        &format!("body\n\n---\n{footer}"),
        &labels_for(REPO),
    )
    .await
    .unwrap();
    kb.comment(&card.id, &format!("ghsync Foodshareclub/{REPO}#1"))
        .await
        .unwrap();
    gh.project_items
        .insert("foodshare-tools#1".to_string(), "I1".to_string());
    gh.project_fields.insert(
        ("I1".to_string(), "Status".to_string()),
        "In progress".to_string(),
    );
    gh.project_fields.insert(
        ("I1".to_string(), "Agent".to_string()),
        "agent-default".to_string(),
    );
    gh.project_fields.insert(
        ("I1".to_string(), "Phase".to_string()),
        "Backlog".to_string(),
    );
    let mut cursor = Cursor::default();
    cursor.set(ISSUE_KEY, "Status", "opt_inprog");
    cursor.set(ISSUE_KEY, "Assignee", "agent-default");
    cursor.set(ISSUE_KEY, "Phase", "ph_backlog");
    (card.id, cursor)
}

async fn report(
    kb: &mut FakeKanban,
    gh: &mut FakeGitHub,
    cursor: &Cursor,
) -> ghsync::status::StatusReport {
    let cfg = test_config();
    run_status(kb, gh, &cfg, cursor).await.unwrap()
}

#[tokio::test]
async fn clean_pair_reports_ok() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let (_, cursor) = clean_pair(&mut kb, &mut gh).await;

    let r = report(&mut kb, &mut gh, &cursor).await;

    assert_eq!(r.rows.len(), 1, "exactly one row for the pair: {r:?}");
    let row = &r.rows[0];
    assert!(row.ok(), "clean pair must be ok, drift: {:?}", row.drift);
    assert_eq!(row.status, "running");
    assert_eq!(row.target, ISSUE_KEY);
    assert_eq!(
        row.labels,
        vec!["ghsync".to_string(), "ghsync-foodshare-tools".to_string()]
    );
    assert_eq!(row.status_field, "In progress");
    assert_eq!(row.assignee_field, "agent-default");
    assert_eq!(row.phase_field, "Backlog");
    assert_eq!(row.milestone, "-");
    assert_eq!(row.pr, "-");
    assert!(r.clean);
}

#[tokio::test]
async fn label_mismatch_flags_exactly() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let (_, cursor) = clean_pair(&mut kb, &mut gh).await;
    // A human (or an old push) left a wrong label set.
    let mut labels = labels_for(REPO);
    labels.pop();
    gh.set_labels(REPO, 1, &labels).await.unwrap();

    let r = report(&mut kb, &mut gh, &cursor).await;

    assert_eq!(r.rows[0].drift, vec!["label-mismatch".to_string()]);
}

#[tokio::test]
async fn field_mismatch_flags_exactly() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let (_, cursor) = clean_pair(&mut kb, &mut gh).await;
    // Someone moved the project card by hand: the field value differs
    // from what ghsync last wrote.
    gh.project_fields.insert(
        ("I1".to_string(), "Status".to_string()),
        "Backlog".to_string(),
    );

    let r = report(&mut kb, &mut gh, &cursor).await;

    assert_eq!(r.rows[0].drift, vec!["field-mismatch".to_string()]);
}

#[tokio::test]
async fn closed_issue_open_card_flags_exactly() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let (_, cursor) = clean_pair(&mut kb, &mut gh).await;
    let key = (REPO.to_string(), 1);
    let issue = gh.issues.get_mut(&key).unwrap();
    issue.state = "closed".into();

    let r = report(&mut kb, &mut gh, &cursor).await;

    assert_eq!(r.rows[0].drift, vec!["closed-but-not-done".to_string()]);
}

#[tokio::test]
async fn missing_footer_flags_exactly() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let (_, cursor) = clean_pair(&mut kb, &mut gh).await;
    // The footer is gone from the issue body; the link still resolves
    // from the card side.
    let key = (REPO.to_string(), 1);
    let issue = gh.issues.get_mut(&key).unwrap();
    issue.body = "body".into();

    let r = report(&mut kb, &mut gh, &cursor).await;

    assert_eq!(r.rows[0].drift, vec!["footer-missing".to_string()]);
}

#[tokio::test]
async fn merged_pr_without_evidence_flags_exactly() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let card = kb
        .create(
            "Fix thing",
            "body\n\nhttps://github.com/Foodshareclub/foodshare-tools/pull/9",
            "default",
            "k1",
            "running",
        )
        .await
        .unwrap();
    let footer = card_link_comment(BOARD, &card.id);
    gh.create_issue(
        REPO,
        "Fix thing",
        &format!("body\n\n---\n{footer}"),
        &labels_for(REPO),
    )
    .await
    .unwrap();
    kb.comment(&card.id, &format!("ghsync Foodshareclub/{REPO}#1"))
        .await
        .unwrap();
    gh.project_items
        .insert("foodshare-tools#1".to_string(), "I1".to_string());
    gh.project_fields.insert(
        ("I1".to_string(), "Status".to_string()),
        "In progress".to_string(),
    );
    gh.project_fields.insert(
        ("I1".to_string(), "Agent".to_string()),
        "agent-default".to_string(),
    );
    gh.project_fields.insert(
        ("I1".to_string(), "Phase".to_string()),
        "Backlog".to_string(),
    );
    gh.pulls.insert(
        (REPO.to_string(), 9),
        Pull {
            number: 9,
            merged: true,
            head_sha: "abc123".into(),
            url: "https://github.com/Foodshareclub/foodshare-tools/pull/9".into(),
        },
    );
    let mut cursor = Cursor::default();
    cursor.set(ISSUE_KEY, "Status", "opt_inprog");
    cursor.set(ISSUE_KEY, "Assignee", "agent-default");
    cursor.set(ISSUE_KEY, "Phase", "ph_backlog");

    let r = report(&mut kb, &mut gh, &cursor).await;

    assert_eq!(r.rows[0].drift, vec!["pr-evidence-missing".to_string()]);
    assert_eq!(r.rows[0].pr, "#9 merged");
}

#[tokio::test]
async fn merged_pr_with_evidence_is_clean() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let card = kb
        .create(
            "Fix thing",
            "body\n\nhttps://github.com/Foodshareclub/foodshare-tools/pull/9",
            "default",
            "k1",
            "running",
        )
        .await
        .unwrap();
    let footer = card_link_comment(BOARD, &card.id);
    gh.create_issue(
        REPO,
        "Fix thing",
        &format!("body\n\n---\n{footer}"),
        &labels_for(REPO),
    )
    .await
    .unwrap();
    kb.comment(&card.id, &format!("ghsync Foodshareclub/{REPO}#1"))
        .await
        .unwrap();
    gh.project_items
        .insert("foodshare-tools#1".to_string(), "I1".to_string());
    gh.project_fields.insert(
        ("I1".to_string(), "Status".to_string()),
        "In progress".to_string(),
    );
    gh.project_fields.insert(
        ("I1".to_string(), "Agent".to_string()),
        "agent-default".to_string(),
    );
    gh.project_fields.insert(
        ("I1".to_string(), "Phase".to_string()),
        "Backlog".to_string(),
    );
    gh.pulls.insert(
        (REPO.to_string(), 9),
        Pull {
            number: 9,
            merged: true,
            head_sha: "abc123".into(),
            url: "https://github.com/Foodshareclub/foodshare-tools/pull/9".into(),
        },
    );
    gh.comment(
        REPO,
        1,
        "PR #9 merged - CI: success - contract: tests green",
    )
    .await
    .unwrap();
    let mut cursor = Cursor::default();
    cursor.set(ISSUE_KEY, "Status", "opt_inprog");
    cursor.set(ISSUE_KEY, "Assignee", "agent-default");
    cursor.set(ISSUE_KEY, "Phase", "ph_backlog");

    let r = report(&mut kb, &mut gh, &cursor).await;

    assert!(
        r.rows[0].ok(),
        "evidence posted: drift: {:?}",
        r.rows[0].drift
    );
}

#[tokio::test]
async fn feed_due_flags_exactly_when_verdict_unposted() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let (card_id, cursor) = clean_pair(&mut kb, &mut gh).await;
    kb.comment(&card_id, "sync:github approve").await.unwrap();

    let r = report(&mut kb, &mut gh, &cursor).await;

    assert_eq!(r.rows[0].drift, vec!["feed-due".to_string()]);

    // After the feed posts, it is no longer due (the verdict line is in
    // the discussion body; the card ref re-derives the link). The
    // seeded discussion sits in the configured feed category (default
    // "General").
    gh.create_discussion(
        REPO,
        "General",
        "[approve] Fix thing",
        &format!(
            "sync:github approve\n\ngsync Foodshareclub/{REPO}#1\n\nCard: kanban:{BOARD}:{card_id}"
        ),
    )
    .await
    .unwrap();
    let r2 = report(&mut kb, &mut gh, &cursor).await;
    assert!(
        r2.rows[0].ok(),
        "posted verdict is not due again: {:?}",
        r2.rows[0].drift
    );
}

#[tokio::test]
async fn unlinked_card_and_orphan_issue_get_their_rows() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let (_, cursor) = clean_pair(&mut kb, &mut gh).await;
    // An unlinked card.
    kb.create("Orphan card", "body", "default", "k2", "ready")
        .await
        .unwrap();
    // An orphan issue: ghsync-labeled, no footer, no card.
    gh.create_issue(REPO, "Orphan issue", "body", &labels_for(REPO))
        .await
        .unwrap();

    let r = report(&mut kb, &mut gh, &cursor).await;

    assert_eq!(r.rows.len(), 3, "pair + unlinked card + orphan: {r:?}");
    let unlinked = r
        .rows
        .iter()
        .find(|row| row.drift == vec!["unlinked".to_string()]);
    assert!(unlinked.is_some(), "unlinked card row: {r:?}");
    let orphan = r
        .rows
        .iter()
        .find(|row| row.drift == vec!["orphan".to_string()]);
    assert!(orphan.is_some(), "orphan issue row: {r:?}");
    assert_eq!(orphan.unwrap().target, "Foodshareclub/foodshare-tools#2");
    assert!(!r.clean);
}

#[tokio::test]
async fn json_variant_serializes_rows() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let (_, cursor) = clean_pair(&mut kb, &mut gh).await;
    let r = report(&mut kb, &mut gh, &cursor).await;

    let v = serde_json::to_value(&r.rows).unwrap();
    assert!(v.is_array());
    let row = &v[0];
    for key in ["task", "status", "target", "labels", "drift"] {
        assert!(row.get(key).is_some(), "missing key {key}: {row:?}");
    }
    assert_eq!(row["clean"], true);
    assert!(r.clean);
}
