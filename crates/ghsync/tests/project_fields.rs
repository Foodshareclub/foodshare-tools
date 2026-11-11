//! T7 — push part 2: Project v2 fields (Status/Assignee/Phase) with
//! cursor dedupe.
//!
//! The cost property under test: steady state (nothing changed) costs zero
//! GraphQL writes *and* zero reads — the cursor file is the only memory.
//! First sight of an (issue, field) pair reads the current value once via
//! the GraphQL node query, then the cursor takes over.

use ghsync::config::{
    Config, ProjConfig, RepoConfig, card_link_comment, issue_link_comment, labels_for,
};
use ghsync::fakes::{FakeGitHub, FakeKanban};
use ghsync::github::GitHub;
use ghsync::kanban::{CardView, Kanban};
use ghsync::link::resolve_links_views;
use ghsync::push::{Cursor, FieldWrite, push_project_fields};

fn test_config() -> Config {
    let p = ProjConfig {
        number: 1,
        status_field: "Status".into(),
        assignee_field: "Assignee".into(),
        phase_field: "Phase".into(),
        assignee_options: [("agent-default", "default"), ("agent-security", "security")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        phase_options: [
            ("Backlog", "backlog"),
            ("In progress", "in-progress"),
            ("Done", "done"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect(),
        status_option_ids: [
            ("Backlog", "opt-backlog"),
            ("In progress", "opt-inprog"),
            ("Done", "opt-done"),
            ("Blocked", "opt-blocked"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect(),
        phase_option_ids: [
            ("Backlog", "ph-backlog"),
            ("In progress", "ph-inprog"),
            ("Done", "ph-done"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect(),
        ..Default::default()
    };
    Config {
        repos: vec![RepoConfig {
            name: "foodshare-tools".into(),
            default_milestone: "Rust + TypeScript".into(),
        }],
        project: Some(p),
        ..Config::default()
    }
}

async fn views(kb: &mut FakeKanban) -> Vec<CardView> {
    let cards = kb.list().await.unwrap();
    let mut out = Vec::new();
    for c in &cards {
        out.push(kb.show(&c.id).await.unwrap());
    }
    out
}

async fn run_fields(
    kb: &mut FakeKanban,
    gh: &mut FakeGitHub,
    cfg: &Config,
    cursor: &mut Cursor,
    dry: bool,
) -> ghsync::push::ProjectReport {
    let vs = views(kb).await;
    let links = resolve_links_views(gh, &cfg.board, &cfg.org, &["foodshare-tools".into()], &vs)
        .await
        .unwrap();
    push_project_fields(gh, cfg, &vs, &links, cursor, dry)
        .await
        .unwrap()
}

/// Board + GitHub in the state a previous push left them: one linked card
/// (ready, assignee `default`), its issue already in the project with
/// matching field values.
async fn linked_board(
    kb: &mut FakeKanban,
    gh: &mut FakeGitHub,
    status: &str,
    assignee: Option<&str>,
) -> String {
    let card = kb
        .create("Card", "x", assignee.unwrap_or("default"), "k1", status)
        .await
        .unwrap();
    // An unclaimed card has no assignee at all.
    if assignee.is_none() {
        kb.cards
            .iter_mut()
            .find(|c| c.id == card.id)
            .unwrap()
            .assignee = None;
    }
    let footer = card_link_comment("foodshare", &card.id);
    gh.create_issue(
        "foodshare-tools",
        "Card",
        &format!("body\n\n---\n{footer}"),
        &labels_for("foodshare-tools"),
    )
    .await
    .unwrap();
    kb.comment(
        &card.id,
        &issue_link_comment("Foodshareclub", "foodshare-tools", 1),
    )
    .await
    .unwrap();
    // The issue is in the project, fields at the values matching `status`
    // (so a first push only records the cursor, it does not write).
    gh.project_items
        .insert("foodshare-tools#1".to_string(), "PRI_1".to_string());
    gh.project_fields
        .insert(("PRI_1".into(), "Status".into()), status_field(status));
    gh.project_fields.insert(
        ("PRI_1".into(), "Assignee".into()),
        "agent-default".to_string(),
    );
    gh.project_fields.insert(
        ("PRI_1".into(), "Phase".into()),
        phase_field(status, assignee).to_string(),
    );
    card.id
}

fn status_field(status: &str) -> String {
    match status {
        "running" | "review" => "opt-inprog".to_string(),
        "blocked" => "opt-blocked".to_string(),
        "done" => "opt-done".to_string(),
        _ => "opt-backlog".to_string(),
    }
}

/// The committed phase rule: in-progress only for a claimed ready card.
fn phase_field(status: &str, assignee: Option<&str>) -> &'static str {
    if status == "ready" && assignee.is_some() {
        "ph-inprog"
    } else {
        "ph-backlog"
    }
}

#[tokio::test]
async fn status_change_between_pushes_is_exactly_one_write() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let cfg = test_config();
    let mut cursor = Cursor::default();
    // todo -> running: only the Status field moves (phase is backlog for
    // both, assignee unchanged).
    let card_id = linked_board(&mut kb, &mut gh, "todo", Some("default")).await;

    // First push: everything matches — cursor is populated, nothing written.
    let r1 = run_fields(&mut kb, &mut gh, &cfg, &mut cursor, false).await;
    assert!(r1.writes.is_empty(), "writes: {:?}", r1.writes);
    assert!(gh.project_field_writes.is_empty());
    assert_eq!(
        cursor.written(&issue_key(), "Status"),
        Some("opt-backlog"),
        "cursor must record the first-sight read"
    );

    // The card moves todo -> running.
    let card = kb.cards.iter_mut().find(|c| c.id == card_id).unwrap();
    card.status = "running".into();

    let r2 = run_fields(&mut kb, &mut gh, &cfg, &mut cursor, false).await;

    assert_eq!(
        r2.writes,
        vec![FieldWrite {
            issue: issue_key(),
            field: "Status".into(),
            from: Some("opt-backlog".into()),
            to: "opt-inprog".into(),
        }]
    );
    assert_eq!(
        gh.project_field_writes,
        vec![(
            "PRI_1".to_string(),
            "Status".to_string(),
            serde_json::json!({ "singleSelectOptionId": "opt-inprog" })
        )]
    );
}

#[tokio::test]
async fn steady_state_costs_zero_graphql_reads_and_writes() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let cfg = test_config();
    let mut cursor = Cursor::default();
    let card_id = linked_board(&mut kb, &mut gh, "running", Some("default")).await;

    run_fields(&mut kb, &mut gh, &cfg, &mut cursor, false).await;
    // Now the board and the cursor agree; simulate the 15-min cron firing
    // again with no changes.
    let reads_before = gh.project_field_reads.len();
    let writes_before = gh.project_field_writes.len();

    let r = run_fields(&mut kb, &mut gh, &cfg, &mut cursor, false).await;

    assert!(r.writes.is_empty());
    assert_eq!(gh.project_field_reads.len(), reads_before);
    assert_eq!(gh.project_field_writes.len(), writes_before);
    let _ = &card_id;
}

#[tokio::test]
async fn assignee_change_is_exactly_one_write() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let cfg = test_config();
    let mut cursor = Cursor::default();
    let card_id = linked_board(&mut kb, &mut gh, "ready", Some("default")).await;
    run_fields(&mut kb, &mut gh, &cfg, &mut cursor, false).await;

    let card = kb.cards.iter_mut().find(|c| c.id == card_id).unwrap();
    card.assignee = Some("security".into());

    let r = run_fields(&mut kb, &mut gh, &cfg, &mut cursor, false).await;

    assert_eq!(
        r.writes,
        vec![FieldWrite {
            issue: issue_key(),
            field: "Assignee".into(),
            from: Some("agent-default".into()),
            to: "agent-security".into(),
        }]
    );
    assert_eq!(
        gh.project_field_writes,
        vec![(
            "PRI_1".to_string(),
            "Assignee".to_string(),
            serde_json::json!({ "text": "agent-security" })
        )]
    );
}

#[tokio::test]
async fn phase_changes_when_card_becomes_claimed() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let cfg = test_config();
    let mut cursor = Cursor::default();
    // Ready but unclaimed: phase is backlog.
    let card_id = linked_board(&mut kb, &mut gh, "ready", None).await;
    run_fields(&mut kb, &mut gh, &cfg, &mut cursor, false).await;

    // Claiming the ready card flips the phase to in-progress.
    let card = kb.cards.iter_mut().find(|c| c.id == card_id).unwrap();
    card.assignee = Some("default".into());

    let r = run_fields(&mut kb, &mut gh, &cfg, &mut cursor, false).await;

    assert_eq!(
        r.writes,
        vec![FieldWrite {
            issue: issue_key(),
            field: "Phase".into(),
            from: Some("ph-backlog".into()),
            to: "ph-inprog".into(),
        }]
    );
    assert_eq!(
        gh.project_field_writes,
        vec![(
            "PRI_1".to_string(),
            "Phase".to_string(),
            serde_json::json!({ "singleSelectOptionId": "ph-inprog" })
        )]
    );
}

#[tokio::test]
async fn issue_missing_from_project_is_added_once() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let cfg = test_config();
    let mut cursor = Cursor::default();
    let card_id = linked_board(&mut kb, &mut gh, "ready", Some("default")).await;
    // The previous push's add did not happen: drop the item from the
    // project (a removed item's field values go with it).
    gh.project_items.remove("foodshare-tools#1");
    gh.project_fields.retain(|(item, _), _| item != "PRI_1");

    let r1 = run_fields(&mut kb, &mut gh, &cfg, &mut cursor, false).await;

    assert_eq!(r1.items_added, vec![issue_key()]);
    let item_id = &gh
        .project_items
        .get("foodshare-tools#1")
        .expect("the issue must now be in the project");
    // All three fields were written to the new item.
    assert_eq!(gh.project_field_writes.len(), 3);
    assert!(
        gh.project_field_writes
            .iter()
            .all(|(item, _, _)| item.as_str() == *item_id),
        "writes: {:?}",
        gh.project_field_writes
    );

    // Second push: the item is there, the cursor is warm — no add, no write.
    let r2 = run_fields(&mut kb, &mut gh, &cfg, &mut cursor, false).await;
    assert!(r2.items_added.is_empty());
    assert!(r2.writes.is_empty());
    let _ = &card_id;
}

#[tokio::test]
async fn dry_run_reports_transitions_without_writing_or_cursoring() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    let cfg = test_config();
    let mut cursor = Cursor::default();
    // Board says running; the project still says backlog (and the issue is
    // not in the project at all).
    linked_board(&mut kb, &mut gh, "running", Some("default")).await;
    gh.project_items.remove("foodshare-tools#1");

    let r = run_fields(&mut kb, &mut gh, &cfg, &mut cursor, true).await;

    assert!(
        r.would_write.iter().any(|w| {
            w.field == "Status" && w.from.as_deref() != Some("opt-inprog") && w.to == "opt-inprog"
        }),
        "would_write: {:?}",
        r.would_write
    );
    assert!(r.writes.is_empty());
    assert!(r.items_added.is_empty());
    assert!(gh.project_field_writes.is_empty());
    assert!(gh.project_items.is_empty());
    assert!(
        cursor.fields.is_empty(),
        "dry run must not touch the cursor"
    );
}

fn issue_key() -> String {
    "Foodshareclub/foodshare-tools#1".to_string()
}
