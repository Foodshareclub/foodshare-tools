//! T9 — intake: `intake`-labeled issues become kanban cards.
//!
//! Runs against the fakes only; the live run is blocked on the org PAT
//! (T0). The rendered form body is pinned here per the plan's form
//! convention (`**<label>:**` headings); T14's live E2E re-verifies the
//! exact rendered text against a real submission.

use ghsync::config::{Config, RepoConfig};
use ghsync::fakes::{FakeGitHub, FakeKanban};
use ghsync::intake::{INTAKE_LABEL, IntakeError, PHASE_AD_HOC, intake_issues, parse_form};

/// The rendered body of the agent-intake form (phase P3, profile
/// architect, multi-line acceptance).
const FORM_BODY: &str = "**Phase:**

P3

**Agent profile:**

architect

**Acceptance criteria:**

cargo test -p ghsync passes
";

/// A plain issue that someone labeled `intake` by hand.
const PLAIN_BODY: &str = "Please fix the flaky test in the CI matrix.";

fn config_with(profiles: &[&str]) -> Config {
    Config {
        repos: vec![RepoConfig {
            name: "foodshare-tools".into(),
            default_milestone: "Rust + TypeScript".into(),
            ..Default::default()
        }],
        profiles: profiles.iter().map(|s| s.to_string()).collect(),
        ..Config::default()
    }
}

/// Seed open issue #7 with the `intake` label and the repo's default
/// milestone (number 3).
fn seed(gh: &mut FakeGitHub, title: &str, body: &str) {
    gh.issues.insert(
        ("foodshare-tools".to_string(), 7),
        ghsync::github::Issue {
            owner: "Foodshareclub".into(),
            repo: "foodshare-tools".into(),
            number: 7,
            title: title.into(),
            body: body.into(),
            node_id: "GH_ISSUE_7".into(),
            state: "open".into(),
            labels: vec![INTAKE_LABEL.to_string()],
            milestone: None,
            url: "https://github.com/Foodshareclub/foodshare-tools/issues/7".into(),
        },
    );
    gh.milestones.insert(
        "foodshare-tools".to_string(),
        vec![ghsync::github::Milestone {
            number: 3,
            title: "Rust + TypeScript".into(),
            open_issues: 0,
            closed_issues: 0,
        }],
    );
}

#[tokio::test]
async fn form_issue_creates_one_card_with_key_assignee_and_milestone() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    seed(&mut gh, "[intake] Fix the flaky test", FORM_BODY);
    let cfg = config_with(&["default", "architect", "reviewer"]);

    let report = intake_issues(&mut kb, &mut gh, &cfg, false).await.unwrap();

    assert_eq!(report.created.len(), 1, "exactly one card");
    let c = &report.created[0];
    assert_eq!(c.issue, "Foodshareclub/foodshare-tools#7");
    assert_eq!(c.profile, "architect", "profile from the form");
    assert_eq!(c.phase, "P3", "phase from the form");

    assert_eq!(kb.created.len(), 1);
    let card = &kb.created[0];
    assert_eq!(card.idempotency_key, "gh-Foodshareclub-foodshare-tools-7");
    assert_eq!(
        card.title,
        "[intake] Fix the flaky test (gh Foodshareclub/foodshare-tools#7)"
    );
    assert_eq!(card.assignee, "architect");
    assert_eq!(
        card.initial_status, "ready",
        "intake cards default to ready"
    );
    // Card body = issue body + the stateless link line.
    assert!(card.body.starts_with(FORM_BODY.trim_end()));
    assert!(
        card.body
            .ends_with("ghsync Foodshareclub/foodshare-tools#7")
    );
    // The card also carries the link comment.
    let comments = kb.comments.get(&card.id).expect("link comment on card");
    assert_eq!(
        comments,
        &vec!["ghsync Foodshareclub/foodshare-tools#7".to_string()]
    );
    // Labels: intake removed, ghsync added.
    let labels = &gh.issues[&("foodshare-tools".to_string(), 7)].labels;
    assert_eq!(labels, &vec!["ghsync".to_string()]);
    // Milestone: the repo's default (form phase P3 != Ad-hoc).
    assert_eq!(
        gh.milestone_set
            .get(&("foodshare-tools".to_string(), 7))
            .copied(),
        Some(3)
    );
    assert!(report.skipped.is_empty());
}

#[tokio::test]
async fn second_run_creates_nothing() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    seed(&mut gh, "[intake] Fix the flaky test", FORM_BODY);
    let cfg = config_with(&["default", "architect"]);

    intake_issues(&mut kb, &mut gh, &cfg, false).await.unwrap();
    let report = intake_issues(&mut kb, &mut gh, &cfg, false).await.unwrap();

    assert_eq!(kb.created.len(), 1, "still exactly one card");
    assert!(report.created.is_empty());
    // After the first run the issue lost its `intake` label, so it no
    // longer lists as intake at all — nothing to skip either.
    assert!(
        report.skipped.is_empty(),
        "relabeled issue is not in the intake set: {report:?}"
    );
    assert!(
        !gh.issues[&("foodshare-tools".to_string(), 7)]
            .labels
            .iter()
            .any(|l| l == INTAKE_LABEL)
    );
}

#[tokio::test]
async fn plain_issue_without_form_sections_uses_defaults() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    seed(&mut gh, "Fix the flaky test", PLAIN_BODY);
    let cfg = config_with(&["default"]);

    let report = intake_issues(&mut kb, &mut gh, &cfg, false).await.unwrap();

    assert_eq!(report.created.len(), 1);
    let c = &report.created[0];
    assert_eq!(c.profile, "default", "missing profile -> config default");
    assert_eq!(c.phase, PHASE_AD_HOC, "missing phase -> Ad-hoc");
    // Ad-hoc: no milestone is set.
    assert!(
        gh.milestone_set.is_empty(),
        "Ad-hoc intake must not set a milestone"
    );
    // The card was still created, with defaults applied.
    assert_eq!(kb.created[0].assignee, "default");
}

#[tokio::test]
async fn unknown_profile_errors_naming_valid_profiles_and_writes_nothing() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    seed(
        &mut gh,
        "[intake] Something",
        "**Agent profile:**

security
",
    );
    let cfg = config_with(&["default", "architect"]);

    let err = intake_issues(&mut kb, &mut gh, &cfg, false)
        .await
        .unwrap_err();
    match err {
        IntakeError::UnknownProfile { profile, valid, .. } => {
            assert_eq!(profile, "security");
            assert_eq!(valid, vec!["default".to_string(), "architect".to_string()]);
        }
        other => panic!("expected UnknownProfile, got: {other}"),
    }
    // Nothing was written.
    assert!(kb.created.is_empty());
    let labels = &gh.issues[&("foodshare-tools".to_string(), 7)].labels;
    assert_eq!(labels, &vec![INTAKE_LABEL.to_string()], "labels untouched");
    assert!(gh.comments_posted.is_empty());
}

#[tokio::test]
async fn dry_run_reports_without_writing() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    seed(&mut gh, "[intake] Fix the flaky test", FORM_BODY);
    let cfg = config_with(&["default", "architect"]);

    let report = intake_issues(&mut kb, &mut gh, &cfg, true).await.unwrap();

    assert_eq!(
        report.would_create,
        vec!["Foodshareclub/foodshare-tools#7".to_string()]
    );
    assert!(report.created.is_empty());
    assert!(kb.created.is_empty());
    let labels = &gh.issues[&("foodshare-tools".to_string(), 7)].labels;
    assert_eq!(labels, &vec![INTAKE_LABEL.to_string()], "labels untouched");
}

#[tokio::test]
async fn closed_intake_issue_is_ignored() {
    let mut kb = FakeKanban::default();
    let mut gh = FakeGitHub::default();
    seed(&mut gh, "[intake] Fix the flaky test", FORM_BODY);
    gh.issues
        .get_mut(&("foodshare-tools".to_string(), 7))
        .unwrap()
        .state = "closed".into();
    let cfg = config_with(&["default", "architect"]);

    let report = intake_issues(&mut kb, &mut gh, &cfg, false).await.unwrap();

    assert!(report.created.is_empty());
    assert!(kb.created.is_empty());
}

#[test]
fn parse_form_reads_all_three_sections() {
    let f = parse_form(FORM_BODY);
    assert_eq!(f.phase.as_deref(), Some("P3"));
    assert_eq!(f.profile.as_deref(), Some("architect"));
    assert_eq!(f.acceptance.as_deref(), Some("cargo test -p ghsync passes"));
}

#[test]
fn parse_form_multiline_acceptance_keeps_newlines() {
    let body = "**Acceptance criteria:**

line one
line two
";
    let f = parse_form(body);
    assert_eq!(f.acceptance.as_deref(), Some("line one\nline two"));
    assert_eq!(f.phase, None);
    assert_eq!(f.profile, None);
}

#[test]
fn parse_form_plain_body_is_all_none() {
    let f = parse_form(PLAIN_BODY);
    assert_eq!(f.phase, None);
    assert_eq!(f.profile, None);
    assert_eq!(f.acceptance, None);
}

/// GitHub's actual issue-form rendering: inline field types (dropdown)
/// put the value on the SAME line as the heading, block types (textarea)
/// put it on the following lines. Pinned against the documented
/// rendering and the live E2E issue (foodshare-tools#217, 2026-11-13 —
/// the first live intake ran through this exact body).
const FORM_BODY_LIVE: &str = "**Phase:** P3

**Agent profile:** default

**Acceptance criteria:**
E2E: exactly one kanban card
";

#[test]
fn parse_form_reads_live_same_line_rendering() {
    let f = parse_form(FORM_BODY_LIVE);
    assert_eq!(f.phase.as_deref(), Some("P3"));
    assert_eq!(f.profile.as_deref(), Some("default"));
    assert_eq!(
        f.acceptance.as_deref(),
        Some("E2E: exactly one kanban card")
    );
}

#[test]
fn parse_form_mixed_renderings_parse_together() {
    // same-line dropdown followed by own-line block field
    let body = "**Phase:** P5\n\n**Acceptance criteria:**\necho ok\n";
    let f = parse_form(body);
    assert_eq!(f.phase.as_deref(), Some("P5"));
    assert_eq!(f.acceptance.as_deref(), Some("echo ok"));
    assert_eq!(f.profile, None);
}
