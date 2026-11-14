//! `ghsync setup`: idempotent org provisioning (plan T8).
//!
//! Every step is find-or-create, so a second run performs zero writes.
//! Steps: (1) verify the token; (2) per-repo labels; (3) the org Project
//! v2 board; (4) the project's fields and single-select options,
//! recording their ids; (5) per-repo default milestones; (6) discussions
//! state report + feed-category verification; (7) project grid views.
//! The CLI then writes the discovered ids back into the config
//! (`apply_report_to_config` + `Config::save`), preserving every key.
//!
//! The live run needs the org token (T0); the unit tests drive this
//! module through the fake with queued GraphQL responses, and the
//! GraphQL strings are snapshot-pinned in `github.rs`.

use std::collections::BTreeMap;

use thiserror::Error;

use crate::config::Config;
use crate::github::{
    GHSYNC_CREATE_KANBAN_VIEW_MUTATION, GHSYNC_CREATE_PHASES_VIEW_MUTATION,
    GHSYNC_CREATE_PROJECT_MUTATION, GHSYNC_CREATE_SINGLE_SELECT_MUTATION,
    GHSYNC_CREATE_TEXT_FIELD_MUTATION, GHSYNC_DISCUSSIONS_QUERY, GHSYNC_FIELDS_QUERY,
    GHSYNC_LIST_PROJECTS_QUERY, GHSYNC_SET_FIELD_OPTIONS_MUTATION, GHSYNC_VIEWS_QUERY, GitHub,
    GitHubError,
};

/// The title of the org Project v2 board.
pub const PROJECT_TITLE: &str = "Foodshare Board";
/// The grid view covering the whole board, grouped by Status.
pub const VIEW_BOARD: &str = "Board";
/// The grid view grouped by Phase.
pub const VIEW_PHASES: &str = "Phases";
/// Label color (decorative).
const LABEL_COLOR: &str = "0ea4a5";

/// A setup failure.
#[derive(Debug, Error)]
pub enum SetupError {
    /// The token was rejected (HTTP 401) — the PAT is missing or wrong.
    #[error(
        "ghsync: token rejected by GitHub (HTTP {status}): {message} — check the PAT in the config"
    )]
    Auth { status: u16, message: String },
    /// The configured feed category does not exist among the repo's
    /// pre-existing discussion categories (categories cannot be created
    /// via API — live-verified).
    #[error(
        "ghsync: repo {repo}: discussion category {wanted:?} not found — available: {available:?} (set `feed_category` on this repo's [[repos]] entry to one of them)"
    )]
    FeedCategory {
        repo: String,
        wanted: String,
        available: Vec<String>,
    },
    /// A required config section is missing.
    #[error("ghsync: {0}")]
    Config(String),
    #[error(transparent)]
    GitHub(#[from] GitHubError),
}

/// What setup found/created; the ids are written back into the config.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SetupReport {
    /// The GitHub login the token authenticated as.
    pub user: String,
    /// (repo, labels created) per repo, in config order.
    pub labels_created: Vec<(String, u32)>,
    /// The project's GraphQL node id (e.g. `PV1_…`).
    pub project_id: String,
    /// True when the project itself was created this run.
    pub project_created: bool,
    /// The Status field id.
    pub status_field: String,
    /// The Assignee field id.
    pub assignee_field: String,
    /// The Phase field id.
    pub phase_field: String,
    /// Status option label -> option id.
    pub status_option_ids: BTreeMap<String, String>,
    /// Phase option label -> option id.
    pub phase_option_ids: BTreeMap<String, String>,
    /// (repo, milestones created) per repo, in config order.
    pub milestones_created: Vec<(String, u32)>,
    /// (repo, human-readable state line) per repo, in config order.
    pub discussions: Vec<(String, String)>,
    /// (repo, discussions enabled, labels supported) per repo, in config
    /// order — written back into the config by the CLI.
    pub discussion_flags: Vec<(String, bool, bool)>,
    /// Names of the views created this run.
    pub views_created: Vec<String>,
}

/// Run every provisioning step against `gh` for `cfg`.
pub async fn run_setup(gh: &mut impl GitHub, cfg: &Config) -> Result<SetupReport, SetupError> {
    let mut report = SetupReport::default();

    // 1. The token.
    let user = gh.get_user().await.map_err(|e| match e {
        GitHubError::Http {
            status, message, ..
        } if status == 401 => SetupError::Auth { status, message },
        other => SetupError::GitHub(other),
    })?;
    report.user = user;

    // 2. Per-repo labels.
    for repo in &cfg.repos {
        let wanted = [
            "ghsync".to_string(),
            format!("ghsync-{}", repo.name),
            "intake".to_string(),
        ];
        let have = gh.list_labels(&repo.name).await?;
        let mut created = 0;
        for name in &wanted {
            if !have.iter().any(|h| h == name) {
                gh.create_label(&repo.name, name, LABEL_COLOR).await?;
                created += 1;
            }
        }
        report.labels_created.push((repo.name.clone(), created));
    }

    // 3. The project (found by title: no projectV2ByName in the GraphQL
    // schema — page through the org's boards, of which there are few).
    // The org node id rides on the same list call: CreateProjectV2Input
    // requires ownerId (live-verified 2026-11-12).
    let mut project_id: Option<String> = None;
    let mut org_id: Option<String> = None;
    let mut cursor: Option<String> = None;
    while project_id.is_none() {
        let data = gh
            .graphql(
                GHSYNC_LIST_PROJECTS_QUERY,
                serde_json::json!({ "org": cfg.org, "cursor": cursor }),
            )
            .await?;
        if org_id.is_none() {
            org_id = data
                .pointer("/organization/id")
                .and_then(|v| v.as_str())
                .map(str::to_string);
        }
        let pv = data
            .pointer("/organization/projectsV2")
            .ok_or_else(|| GitHubError::Io("projectsV2: missing from response".into()))?;
        let nodes = pv
            .pointer("/nodes")
            .and_then(|n| n.as_array())
            .cloned()
            .unwrap_or_default();
        for node in &nodes {
            if node.pointer("/title").and_then(|t| t.as_str()) == Some(PROJECT_TITLE) {
                project_id = node
                    .pointer("/id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                break;
            }
        }
        let more = pv
            .pointer("/pageInfo/hasNextPage")
            .and_then(|h| h.as_bool())
            .unwrap_or(false);
        let next = pv
            .pointer("/pageInfo/endCursor")
            .and_then(|c| c.as_str())
            .map(str::to_string);
        if !more || next.is_none() {
            break;
        }
        cursor = next;
    }
    let project_id = match project_id {
        Some(id) => id.to_string(),
        None => {
            let org_id = org_id.ok_or_else(|| {
                GitHubError::Io("organization id missing from projects list response".into())
            })?;
            let data = gh
                .graphql(
                    GHSYNC_CREATE_PROJECT_MUTATION,
                    serde_json::json!({
                        "input": {
                            "ownerId": org_id,
                            "title": PROJECT_TITLE
                        }
                    }),
                )
                .await?;
            report.project_created = true;
            data.pointer("/createProjectV2/projectV2/id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| {
                    GitHubError::Io("createProjectV2: response missing project id".into())
                })?
        }
    };
    report.project_id = project_id.clone();

    // 4. Fields + options.
    let data = gh
        .graphql(
            GHSYNC_FIELDS_QUERY,
            serde_json::json!({ "projectId": project_id }),
        )
        .await?;
    // Index the fields by BOTH id and name. Pre-setup the config's
    // field entries hold names; after a run's write-back they hold ids.
    // A ghost field (created by the old name-only lookup, live incident
    // 2026-11-13) is NAMED after the previous revision's field id, so an
    // id must resolve by id — a name match on an id-shaped value would
    // adopt the ghost.
    let mut by_id: FieldIndex = BTreeMap::new();
    let mut by_name: FieldIndex = BTreeMap::new();
    if let Some(nodes) = data
        .pointer("/node/fields/nodes")
        .and_then(|n| n.as_array())
    {
        for n in nodes {
            let id = n
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let name = n
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let options: Vec<(String, String)> = n
                .get("options")
                .and_then(|o| o.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|o| {
                            Some((
                                o.get("id")?.as_str()?.to_string(),
                                o.get("name")?.as_str()?.to_string(),
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default();
            by_id.insert(id.clone(), (id.clone(), options.clone()));
            by_name.insert(name, (id, options));
        }
    }

    let proj = cfg.project.as_ref();
    let status_keys: Vec<String> = proj
        .map(|p| p.status_options.keys().cloned().collect())
        .unwrap_or_default();
    let phase_keys: Vec<String> = proj
        .map(|p| p.phase_options.keys().cloned().collect())
        .unwrap_or_default();

    // 4. Status / Agent / Phase fields. The board field names come from
    // the config (`status_field`/`assignee_field`/`phase_field` hold the
    // names until setup has replaced them with the live field ids).
    // "Assignee" is a reserved custom-field name in the live schema
    // (live-verified 2026-11-13), which is why the default board name
    // is "Agent".
    let Some(proj) = cfg.project.as_ref() else {
        return Err(SetupError::Config(
            "missing [project] section — cannot run setup".into(),
        ));
    };
    let status_name = proj.status_field.clone();
    let assignee_name = proj.assignee_field.clone();
    let phase_name = proj.phase_field.clone();
    let existing_status = resolve_field(&status_name, &by_id, &by_name)?;
    let assignee_field = match resolve_field(&assignee_name, &by_id, &by_name)? {
        Some((id, _)) => id,
        None => {
            let data = gh
                .graphql(
                    GHSYNC_CREATE_TEXT_FIELD_MUTATION,
                    serde_json::json!({ "projectId": project_id, "name": assignee_name }),
                )
                .await?;
            data.pointer("/createProjectV2Field/projectV2Field/id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| {
                    GitHubError::Io("createProjectV2Field(Assignee): missing id".into())
                })?
        }
    };
    report.assignee_field = assignee_field;
    let existing_phase = resolve_field(&phase_name, &by_id, &by_name)?;
    let status_field =
        ensure_single_select(gh, &project_id, &status_name, &status_keys, existing_status).await?;
    report.status_field = status_field.clone();
    let phase_field =
        ensure_single_select(gh, &project_id, &phase_name, &phase_keys, existing_phase).await?;
    report.phase_field = phase_field.clone();

    // Option ids: the create/update responses do not return the options,
    // so one more fields read collects them (setup is a one-off; the
    // steady-state cost lives in push, not here). Matched by field id —
    // a ghost field is named after a field id, and a name match would
    // collect the ghost's options.
    let data = gh
        .graphql(
            GHSYNC_FIELDS_QUERY,
            serde_json::json!({ "projectId": project_id }),
        )
        .await?;
    report.status_option_ids = option_ids_from(&data, &status_field);
    report.phase_option_ids = option_ids_from(&data, &phase_field);

    // 5. Per-repo default milestones.
    for repo in &cfg.repos {
        let ms = gh.list_milestones(&repo.name).await?;
        if !ms.iter().any(|m| m.title == repo.default_milestone) {
            gh.create_milestone(&repo.name, &repo.default_milestone)
                .await?;
            report.milestones_created.push((repo.name.clone(), 1));
        } else {
            report.milestones_created.push((repo.name.clone(), 0));
        }
    }

    // 6. Discussions: report state, verify the configured feed category.
    // `hasDiscussions` does not exist on Repository in GraphQL
    // (live-verified), so the enabled flag comes from the REST repo
    // object; categories cannot be created via API at all
    // (live-verified), so the feed category must pre-exist — a missing
    // one is a config error naming the available list.
    for repo in &cfg.repos {
        let enabled = gh.has_discussions(&repo.name).await?;
        if !enabled {
            report.discussions.push((
                repo.name.clone(),
                "disabled — discussion intake inert".into(),
            ));
            report
                .discussion_flags
                .push((repo.name.clone(), false, false));
            continue;
        }
        let data = gh
            .graphql(
                GHSYNC_DISCUSSIONS_QUERY,
                serde_json::json!({ "owner": cfg.org, "name": &repo.name }),
            )
            .await?;
        let categories = data
            .pointer("/repository/discussionCategories/nodes")
            .and_then(|n| n.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c.get("name").and_then(|v| v.as_str()))
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !categories.iter().any(|c| c == &repo.feed_category) {
            return Err(SetupError::FeedCategory {
                repo: repo.name.clone(),
                wanted: repo.feed_category.clone(),
                available: categories,
            });
        }
        report.discussions.push((
            repo.name.clone(),
            format!("enabled — feed category {} present", repo.feed_category),
        ));
        // Probe label support via the repo's labels route (404/410/empty
        // = unsupported) and record the flags; the feed/intake steps
        // gate on them.
        let labels_ok = gh.discussion_labels_supported(&repo.name).await?;
        report
            .discussion_flags
            .push((repo.name.clone(), true, labels_ok));
    }

    // 7. Project grid views.
    let data = gh
        .graphql(
            GHSYNC_VIEWS_QUERY,
            serde_json::json!({ "projectId": project_id }),
        )
        .await?;
    let views: Vec<String> = data
        .pointer("/node/views/nodes")
        .and_then(|n| n.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("name").and_then(|x| x.as_str()))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if !views.iter().any(|v| v == VIEW_BOARD) {
        gh.graphql(
            GHSYNC_CREATE_KANBAN_VIEW_MUTATION,
            serde_json::json!({ "projectId": project_id, "name": VIEW_BOARD }),
        )
        .await?;
        report.views_created.push(VIEW_BOARD.into());
    }
    if !views.iter().any(|v| v == VIEW_PHASES) {
        gh.graphql(
            GHSYNC_CREATE_PHASES_VIEW_MUTATION,
            serde_json::json!({ "projectId": project_id, "name": VIEW_PHASES }),
        )
        .await?;
        report.views_created.push(VIEW_PHASES.into());
    }

    Ok(report)
}

/// One field from a fields read: (field id, option id -> option name).
type FieldEntry = (String, Vec<(String, String)>);
/// Fields indexed by id or by name (the config's field entry is a name
/// pre-setup and an id after a run's write-back).
type FieldIndex = BTreeMap<String, FieldEntry>;

/// The live id shapes of the two custom project field kinds
/// (text / single-select), live-verified 2026-11-13.
fn is_field_id(value: &str) -> bool {
    value.starts_with("PVTF_") || value.starts_with("PVTSSF_")
}

/// Resolve one configured field entry against a fields read.
///
/// Pre-setup the entry holds the field's NAME (the provisioning
/// default); after a run's write-back it holds the field's ID. An
/// id-shaped value resolves by id only — if the id is gone (the board
/// changed out from under the config) the run fails loudly instead of
/// creating a field named after the id. That silent create is how the
/// ghost chain on the Foodshare Board started: each re-run adopted — or
/// created — the field named after the previous revision's id.
fn resolve_field(
    wanted: &str,
    by_id: &FieldIndex,
    by_name: &FieldIndex,
) -> Result<Option<FieldEntry>, SetupError> {
    if is_field_id(wanted) {
        return by_id
            .get(wanted)
            .cloned()
            .map(Some)
            .ok_or_else(|| {
                SetupError::Config(format!(
                    "configured field id {wanted} does not exist in the project — the board changed out from under the config; inspect the board, then re-run setup (do not re-create the field by name)"
                ))
            });
    }
    Ok(by_name.get(wanted).cloned())
}

/// Ensure a single-select field exists with all the wanted options;
/// returns the field id.
///
/// Live-verified 2026-11-13: there is no per-option mutation — options
/// ride on the field create, and a field missing options is updated once
/// with the FULL merged list (existing options keep their ids, which
/// items in the project reference). Option ids are collected by the
/// caller from a final fields read.
async fn ensure_single_select(
    gh: &mut impl GitHub,
    project_id: &str,
    name: &str,
    option_names: &[String],
    existing: Option<(String, Vec<(String, String)>)>,
) -> Result<String, GitHubError> {
    match existing {
        Some((field_id, existing_options)) => {
            let missing: Vec<&String> = option_names
                .iter()
                .filter(|want| !existing_options.iter().any(|(_, n)| n == *want))
                .collect();
            if missing.is_empty() {
                return Ok(field_id);
            }
            // Merged list: existing options first (ids preserved), then
            // the new ones.
            let options: Vec<serde_json::Value> = existing_options
                .iter()
                .map(|(id, n)| option_input(Some(id), n))
                .chain(missing.iter().map(|n| option_input(None, n)))
                .collect();
            gh.graphql(
                GHSYNC_SET_FIELD_OPTIONS_MUTATION,
                serde_json::json!({ "fieldId": field_id, "options": options }),
            )
            .await?;
            Ok(field_id)
        }
        None => {
            let options: Vec<serde_json::Value> =
                option_names.iter().map(|n| option_input(None, n)).collect();
            let data = gh
                .graphql(
                    GHSYNC_CREATE_SINGLE_SELECT_MUTATION,
                    serde_json::json!({
                        "projectId": project_id,
                        "name": name,
                        "options": options
                    }),
                )
                .await?;
            data.pointer("/createProjectV2Field/projectV2Field/id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| GitHubError::Io(format!("createProjectV2Field({name}): missing id")))
        }
    }
}

/// One option input. The live `ProjectV2SingleSelectFieldOptionInput`
/// requires name, color and description (live-verified 2026-11-13); an
/// existing option keeps its id.
fn option_input(id: Option<&str>, name: &str) -> serde_json::Value {
    let mut v = serde_json::json!({
        "name": name,
        "color": option_color(name),
        "description": ""
    });
    if let Some(id) = id {
        v["id"] = serde_json::json!(id);
    }
    v
}

/// Cosmetic board column colors; the enum values are live-verified
/// (GRAY/BLUE/RED/GREEN).
fn option_color(name: &str) -> &'static str {
    match name {
        "In progress" => "BLUE",
        "Blocked" => "RED",
        "Done" => "GREEN",
        _ => "GRAY",
    }
}

/// Option label -> option id for one field, from a fields-read
/// response. Matched by the field's ID: a ghost field is NAMED after a
/// field id, so a name match would collect the ghost's options.
fn option_ids_from(data: &serde_json::Value, field_id: &str) -> BTreeMap<String, String> {
    data.pointer("/node/fields/nodes")
        .and_then(|n| n.as_array())
        .map(|nodes| {
            nodes
                .iter()
                .find(|n| n.get("id").and_then(|v| v.as_str()) == Some(field_id))
                .and_then(|n| n.get("options").and_then(|o| o.as_array()))
                .map(|arr| {
                    arr.iter()
                        .filter_map(|o| {
                            Some((
                                o.get("name")?.as_str()?.to_string(),
                                o.get("id")?.as_str()?.to_string(),
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

/// Copy the discovered ids into the config so the next `Config::load`
/// sees them (the caller saves the config afterwards).
pub fn apply_report_to_config(cfg: &mut Config, report: &SetupReport) {
    if let Some(p) = cfg.project.as_mut() {
        p.project_id = report.project_id.clone();
        p.status_field = report.status_field.clone();
        p.assignee_field = report.assignee_field.clone();
        p.phase_field = report.phase_field.clone();
        p.status_option_ids = report.status_option_ids.clone();
        p.phase_option_ids = report.phase_option_ids.clone();
    }
    for (repo, enabled, labels) in &report.discussion_flags {
        if let Some(r) = cfg.repos.iter_mut().find(|r| &r.name == repo) {
            r.discussions_enabled = *enabled;
            r.discussion_labels_enabled = *labels;
        }
    }
}
