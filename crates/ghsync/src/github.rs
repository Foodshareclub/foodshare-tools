//! The GitHub seam: what ghsync can do to the org.
//!
//! [`GitHub`] is the interface for both the REST endpoints (issues,
//! milestones, comments) and the GraphQL endpoint (Project v2,
//! discussions). [`RealGitHub`] implements it over `reqwest`;
//! [`crate::fakes::FakeGitHub`] is the in-memory stand-in used by the
//! tests.
//!
//! The request surface is built from pure path builders, which are
//! snapshot-tested here; the live wire format is exercised in the E2E.

use thiserror::Error;

/// A GitHub issue (not a PR).
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub owner: String,
    pub repo: String,
    pub number: u32,
    pub title: String,
    pub body: String,
    /// GitHub's node id (`GH_ISSUE_…`); the content id for project items.
    pub node_id: String,
    /// `open` or `closed`.
    pub state: String,
    pub labels: Vec<String>,
    /// The issue's milestone title, if one is set.
    pub milestone: Option<String>,
    pub url: String,
}

/// Pull request info (merge state + head sha).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pull {
    pub number: u32,
    /// Whether the PR has been merged.
    pub merged: bool,
    /// The head commit's sha (the CI rollup is read against it).
    pub head_sha: String,
    pub url: String,
}

/// One check run against a commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckRun {
    /// `completed` or `in_progress` (and friends).
    pub status: String,
    /// `None` while running; then `success`, `failure`, `cancelled`, ...
    pub conclusion: Option<String>,
}

/// A milestone.
#[derive(Debug, Clone, PartialEq)]
pub struct Milestone {
    pub number: u32,
    pub title: String,
    pub open_issues: u32,
    pub closed_issues: u32,
}

/// An issue comment.
#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    pub id: u64,
    pub user: String,
    pub body: String,
    pub created_at: String,
}

/// A label on a discussion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscussionLabel {
    pub id: String,
    pub name: String,
}

/// A discussion. `id` is the GraphQL node id (used by the comment and
/// label mutations); `number` is the human-facing `#n`.
#[derive(Debug, Clone, PartialEq)]
pub struct Discussion {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub body: String,
    pub labels: Vec<DiscussionLabel>,
    /// Comment bodies, oldest first (used for dedupe + append detection).
    pub comments: Vec<String>,
}

/// Failure of a GitHub operation.
#[derive(Debug, Error)]
pub enum GitHubError {
    #[error("github http {status} for {url}: {message}")]
    Http {
        status: u16,
        url: String,
        message: String,
    },
    #[error("github graphql error for {url}: {message}")]
    Graphql { url: String, message: String },
    #[error("github io: {0}")]
    Io(String),
}

/// The GitHub surface ghsync talks to.
///
/// `async_fn_in_trait` is allowed deliberately: this is an internal trait
/// (the crate's own implementations and tests are the only callers), and
/// every implementation returns a `Send` future anyway.
#[allow(async_fn_in_trait)]
pub trait GitHub: Send + Sync {
    /// Create an issue in `repo` (the org from the config).
    async fn create_issue(
        &mut self,
        repo: &str,
        title: &str,
        body: &str,
        labels: &[String],
    ) -> Result<Issue, GitHubError>;
    /// Fetch one issue.
    async fn get_issue(&mut self, repo: &str, number: u32) -> Result<Issue, GitHubError>;
    /// All comments on an issue.
    async fn issue_comments(
        &mut self,
        repo: &str,
        number: u32,
    ) -> Result<Vec<Comment>, GitHubError>;
    /// Create a milestone in `repo`.
    async fn create_milestone(&mut self, repo: &str, title: &str)
    -> Result<Milestone, GitHubError>;
    /// All milestones in `repo` (open and closed).
    async fn list_milestones(&mut self, repo: &str) -> Result<Vec<Milestone>, GitHubError>;
    /// Set the milestone on an issue.
    async fn set_milestone(
        &mut self,
        repo: &str,
        number: u32,
        milestone: u32,
    ) -> Result<(), GitHubError>;
    /// Replace the label set on an issue.
    async fn set_labels(
        &mut self,
        repo: &str,
        number: u32,
        labels: &[String],
    ) -> Result<(), GitHubError>;
    /// Close an issue (never reopen).
    async fn close_issue(&mut self, repo: &str, number: u32) -> Result<(), GitHubError>;
    /// Replace the issue body (e.g. re-appending a link footer a human
    /// stripped).
    async fn set_issue_body(
        &mut self,
        repo: &str,
        number: u32,
        body: &str,
    ) -> Result<(), GitHubError>;
    /// All issues (open and closed) in `repo` carrying `label`, excluding
    /// pull requests.
    async fn issues_labeled(&mut self, repo: &str, label: &str) -> Result<Vec<Issue>, GitHubError>;
    /// Post a comment on an issue.
    async fn comment(&mut self, repo: &str, number: u32, body: &str) -> Result<(), GitHubError>;
    /// Run a GraphQL query; `data` of the response is returned. A
    /// response with `errors` is a [`GitHubError::Graphql`].
    async fn graphql(
        &mut self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value, GitHubError>;
    /// The id of the project item for the issue `repo`/`number`, if the
    /// issue is in the project `project_id`.
    async fn project_item_for_issue(
        &mut self,
        project_id: &str,
        repo: &str,
        number: u32,
    ) -> Result<Option<String>, GitHubError>;
    /// Add the issue (by node id) to the project; returns the new item id.
    async fn add_project_item(
        &mut self,
        project_id: &str,
        node_id: &str,
    ) -> Result<String, GitHubError>;
    /// The current value of an item's field: the option name for
    /// single-select fields, the text for text fields; `None` if unset.
    /// (The live query reads the `fieldValues` connection and matches
    /// on the field's node id — see `GHSYNC_FIELD_VALUE_QUERY`.)
    async fn project_field_value(
        &mut self,
        item_id: &str,
        field_id: &str,
    ) -> Result<Option<String>, GitHubError>;
    /// Set an item's field. `value` is the GraphQL field-value input,
    /// e.g. `{"singleSelectOptionId": "…"}` or `{"text": "…"}` (the
    /// live `ProjectV2FieldValue` input object; the input also requires
    /// the project id).
    async fn set_project_field(
        &mut self,
        project_id: &str,
        item_id: &str,
        field_id: &str,
        value: &serde_json::Value,
    ) -> Result<(), GitHubError>;
    /// The login of the authenticated user; a 401 is a `GitHubError::Http`.
    async fn get_user(&mut self) -> Result<String, GitHubError>;
    /// All label names in `repo`.
    async fn list_labels(&mut self, repo: &str) -> Result<Vec<String>, GitHubError>;
    /// Create a label in `repo` (the color is decorative).
    async fn create_label(
        &mut self,
        repo: &str,
        name: &str,
        color: &str,
    ) -> Result<(), GitHubError>;
    /// Whether GitHub Discussions are enabled on `repo` (REST: the
    /// `has_discussions` field of the repo object; GraphQL's
    /// `hasDiscussions` does not exist — live-verified).
    async fn has_discussions(&mut self, repo: &str) -> Result<bool, GitHubError>;
    /// Pull request info (merge state + head sha).
    async fn get_pull(&mut self, repo: &str, number: u32) -> Result<Pull, GitHubError>;
    /// The check runs recorded against a commit.
    async fn check_runs(&mut self, repo: &str, sha: &str) -> Result<Vec<CheckRun>, GitHubError>;
    /// Discussions in `repo`, filtered by category name and/or label name
    /// (both optional).
    async fn list_discussions(
        &mut self,
        repo: &str,
        category: Option<&str>,
        label: Option<&str>,
    ) -> Result<Vec<Discussion>, GitHubError>;
    /// Create a discussion in the named category.
    async fn create_discussion(
        &mut self,
        repo: &str,
        category: &str,
        title: &str,
        body: &str,
    ) -> Result<Discussion, GitHubError>;
    /// Add a comment to a discussion.
    async fn discussion_comment(
        &mut self,
        repo: &str,
        discussion_id: &str,
        body: &str,
    ) -> Result<(), GitHubError>;
    /// Remove a label (by id) from a discussion.
    async fn remove_discussion_label(
        &mut self,
        repo: &str,
        discussion_id: &str,
        label_id: &str,
    ) -> Result<(), GitHubError>;
    /// Whether the repo supports discussion labels (the probe setup
    /// records; 404 = disabled).
    async fn discussion_labels_supported(&mut self, repo: &str) -> Result<bool, GitHubError>;
}

// ---------- Project v2 GraphQL (snapshot-pinned) ----------
//
// Project v2 is GraphQL-only and the strings below are the contract with
// the live API: the pinned tests in `tests` fail on any drift.

/// List a project's items, paged.
pub const GHSYNC_ITEMS_QUERY: &str = r#"query GhsyncProjectItems($projectId: ID!, $cursor: String) {
  node(id: $projectId) {
    ... on ProjectV2 {
      items(first: 100, after: $cursor) {
        pageInfo {
          hasNextPage
          endCursor
        }
        nodes {
          id
          content {
            ... on Issue {
              number
              repository {
                name
              }
            }
          }
        }
      }
    }
  }
}"#;

/// Read the current value of one project item field.
///
/// Live-verified 2026-11-13: `ProjectV2Item` has no
/// `fieldValueByFieldId` — the value is read from the `fieldValues`
/// connection and matched on the field's node id, which each value
/// carries via the `field` selection (a `ProjectV2FieldConfiguration`
/// union: fragments only).
pub const GHSYNC_FIELD_VALUE_QUERY: &str = r#"query GhsyncFieldValue($itemId: ID!) {
  node(id: $itemId) {
    ... on ProjectV2Item {
      fieldValues(first: 100) {
        nodes {
          ... on ProjectV2ItemFieldSingleSelectValue {
            name
            field {
              ... on ProjectV2SingleSelectField {
                id
              }
            }
          }
          ... on ProjectV2ItemFieldTextValue {
            text
            field {
              ... on ProjectV2Field {
                id
              }
            }
          }
        }
      }
    }
  }
}"#;

/// Set a project item field value.
///
/// Live-verified 2026-11-13: the value is the `ProjectV2FieldValue`
/// input object (`{singleSelectOptionId | text | …}`) and the input
/// also requires `projectId`.
pub const GHSYNC_SET_FIELD_MUTATION: &str = r#"mutation GhsyncSetField($projectId: ID!, $itemId: ID!, $fieldId: ID!, $value: ProjectV2FieldValue!) {
  updateProjectV2ItemFieldValue(input: {projectId: $projectId, itemId: $itemId, fieldId: $fieldId, value: $value}) {
    projectV2Item {
      id
    }
  }
}"#;

/// Add an issue to a project by its node id.
pub const GHSYNC_ADD_ITEM_MUTATION: &str = r#"mutation GhsyncAddItem($projectId: ID!, $contentId: ID!) {
  addProjectV2ItemById(input: {projectId: $projectId, contentId: $contentId}) {
    item {
      id
    }
  }
}"#;

// ---------- setup GraphQL (snapshot-pinned) ----------

/// List the org's projects (paginated). GitHub's GraphQL has no
/// `projectV2ByName`; the org has few boards, so setup pages through and
/// matches the title itself.
pub const GHSYNC_LIST_PROJECTS_QUERY: &str = r#"query GhsyncListProjects($org: String!, $cursor: String) {
  organization(login: $org) {
    id
    projectsV2(first: 100, after: $cursor) {
      nodes {
        id
        title
        number
      }
      pageInfo {
        hasNextPage
        endCursor
      }
    }
  }
}"#;

/// Create the org project (org-wide content).
pub const GHSYNC_CREATE_PROJECT_MUTATION: &str = r#"mutation GhsyncCreateProject($input: CreateProjectV2Input!) {
  createProjectV2(input: $input) {
    projectV2 {
      id
      title
    }
  }
}"#;

/// List a project's fields with their single-select options.
///
/// Live-verified 2026-11-13: `fields.nodes` is the
/// `ProjectV2FieldConfiguration` union — selections are fragments only.
/// Text/number fields are the `ProjectV2Field` object; single-selects
/// are `ProjectV2SingleSelectField` (the old
/// `ProjectV2FieldSingleSelect` name does not exist).
pub const GHSYNC_FIELDS_QUERY: &str = r#"query GhsyncProjectFields($projectId: ID!) {
  node(id: $projectId) {
    ... on ProjectV2 {
      fields(first: 100) {
        nodes {
          ... on ProjectV2Field {
            id
            name
          }
          ... on ProjectV2SingleSelectField {
            id
            name
            options {
              id
              name
            }
          }
        }
      }
    }
  }
}"#;

/// Create a single-select field, with its options.
///
/// Live-verified 2026-11-13: there is no separate option-create
/// mutation — options ride on the field create, each as a
/// `ProjectV2SingleSelectFieldOptionInput` (name + color + description
/// all required). The payload field is `projectV2Field` (a union:
/// fragments only).
pub const GHSYNC_CREATE_SINGLE_SELECT_MUTATION: &str = r#"mutation GhsyncCreateSingleSelect($projectId: ID!, $name: String!, $options: [ProjectV2SingleSelectFieldOptionInput!]!) {
  createProjectV2Field(input: {projectId: $projectId, dataType: SINGLE_SELECT, name: $name, singleSelectOptions: $options}) {
    projectV2Field {
      ... on ProjectV2SingleSelectField {
        id
      }
    }
  }
}"#;

/// Create a text field.
pub const GHSYNC_CREATE_TEXT_FIELD_MUTATION: &str = r#"mutation GhsyncCreateTextField($projectId: ID!, $name: String!) {
  createProjectV2Field(input: {projectId: $projectId, dataType: TEXT, name: $name, singleSelectOptions: []}) {
    projectV2Field {
      ... on ProjectV2Field {
        id
      }
    }
  }
}"#;

/// Replace a single-select field's options with the given FULL list.
///
/// Live-verified 2026-11-13: options cannot be added individually — the
/// update carries the whole list; existing options keep their ids (items
/// in the project reference them), new ones are appended without.
pub const GHSYNC_SET_FIELD_OPTIONS_MUTATION: &str = r#"mutation GhsyncSetFieldOptions($fieldId: ID!, $options: [ProjectV2SingleSelectFieldOptionInput!]!) {
  updateProjectV2Field(input: {fieldId: $fieldId, singleSelectOptions: $options}) {
    projectV2Field {
      ... on ProjectV2SingleSelectField {
        id
      }
    }
  }
}"#;

/// List a project's views.
pub const GHSYNC_VIEWS_QUERY: &str = r#"query GhsyncProjectViews($projectId: ID!) {
  node(id: $projectId) {
    ... on ProjectV2 {
      views(first: 100) {
        nodes {
          id
          name
        }
      }
    }
  }
}"#;

/// Create the kanban "Board" view.
///
/// Live-verified 2026-11-13: the view input takes `name` (not
/// `viewName`) and a `layout` enum — the live schema has no KANBAN
/// view type; BOARD_LAYOUT is the board grid. The payload field is
/// `projectV2View`.
pub const GHSYNC_CREATE_KANBAN_VIEW_MUTATION: &str = r#"mutation GhsyncCreateKanbanView($projectId: ID!, $name: String!) {
  createProjectV2View(input: {projectId: $projectId, name: $name, layout: BOARD_LAYOUT, configuration: {visibleFieldIds: []}}) {
    projectV2View {
      id
    }
  }
}"#;

/// Create the "Phases" view over the Phase field.
///
/// Live-verified 2026-11-13: there is no PHASES layout in the live
/// schema, and "Roadmap views do not support visible fields" — so the
/// Phases view is a plain ROADMAP_LAYOUT with no configuration.
pub const GHSYNC_CREATE_PHASES_VIEW_MUTATION: &str = r#"mutation GhsyncCreatePhasesView($projectId: ID!, $name: String!) {
  createProjectV2View(input: {projectId: $projectId, name: $name, layout: ROADMAP_LAYOUT}) {
    projectV2View {
      id
    }
  }
}"#;

/// A repo's discussion categories. The enabled flag is NOT part of this
/// query: `hasDiscussions` does not exist on Repository (live-verified)
/// and comes from REST `GET /repos/{org}/{repo}` instead. The connection
/// requires `first` (a bare selection returns a null repository,
/// live-verified).
pub const GHSYNC_DISCUSSIONS_QUERY: &str = r#"query GhsyncDiscussions($owner: String!, $name: String!) {
  repository(owner: $owner, name: $name) {
    id
    discussionCategories(first: 100) {
      nodes {
        id
        name
      }
    }
  }
}"#;

/// List a repo's discussions. The `category`/`label` arguments do not
/// exist on `Repository.discussions` (live-verified), so the query pulls
/// the first page and the caller filters on the selected category and
/// label names.
pub const GHSYNC_DISCUSSION_LIST_QUERY: &str = r#"query GhsyncDiscussionList($owner: String!, $name: String!) {
  repository(owner: $owner, name: $name) {
    discussions(first: 100) {
      nodes {
        id
        number
        title
        body
        category {
          id
          name
        }
        labels(first: 20) {
          nodes {
            id
            name
          }
        }
        comments(first: 100) {
          nodes {
            body
          }
        }
      }
    }
  }
}"#;

/// Create a discussion in an existing category. `addDiscussion` does not
/// exist (live-verified); categories themselves cannot be created via
/// API, so the category must pre-exist.
pub const GHSYNC_CREATE_DISCUSSION_MUTATION: &str = r#"mutation GhsyncCreateDiscussion($repositoryId: ID!, $categoryId: ID!, $title: String!, $body: String!) {
  createDiscussion(input: {repositoryId: $repositoryId, categoryId: $categoryId, title: $title, body: $body}) {
    discussion {
      id
      number
      title
    }
  }
}"#;

/// Add a comment to a discussion.
pub const GHSYNC_DISCUSSION_COMMENT_MUTATION: &str = r#"mutation GhsyncDiscussionComment($discussionId: ID!, $body: String!) {
  addDiscussionComment(input: {discussionId: $discussionId, body: $body}) {
    comment {
      id
    }
  }
}"#;

/// Remove a label from a discussion. `removeDiscussionLabel` does not
/// exist (live-verified); label operations on a discussion go through
/// the shared labelable mutations (verified live: the repo's issue
/// labels attach to discussions through this path).
pub const GHSYNC_REMOVE_DISCUSSION_LABEL_MUTATION: &str = r#"mutation GhsyncRemoveDiscussionLabel($labelableId: ID!, $labelIds: [ID!]!) {
  removeLabelsFromLabelable(input: {labelableId: $labelableId, labelIds: $labelIds}) {
    clientMutationId
  }
}"#;

// ---------- pure path builders (snapshot-tested) ----------

/// `/repos/{org}/{repo}`
pub fn path_repo(org: &str, repo: &str) -> String {
    format!("/repos/{org}/{repo}")
}

/// `/repos/{org}/{repo}/issues/{n}`
pub fn path_issue(org: &str, repo: &str, number: u32) -> String {
    format!("/repos/{org}/{repo}/issues/{number}")
}

/// `/repos/{org}/{repo}/issues/{n}/comments?per_page=100&page={p}`
pub fn path_issue_comments(org: &str, repo: &str, number: u32, page: u32) -> String {
    format!("/repos/{org}/{repo}/issues/{number}/comments?per_page=100&page={page}")
}

/// `/repos/{org}/{repo}/milestones?state=all&per_page=100`
pub fn path_milestones(org: &str, repo: &str) -> String {
    format!("/repos/{org}/{repo}/milestones?state=all&per_page=100")
}

/// `/repos/{org}/{repo}/issues?labels={label}&state=all&per_page=100&page={p}`
pub fn path_issues_labeled(org: &str, repo: &str, label: &str, page: u32) -> String {
    format!("/repos/{org}/{repo}/issues?labels={label}&state=all&per_page=100&page={page}")
}

/// A single pull request.
pub fn path_pull(org: &str, repo: &str, number: u32) -> String {
    format!("/repos/{org}/{repo}/pulls/{number}")
}

/// The check runs against a commit.
pub fn path_check_runs(org: &str, repo: &str, sha: &str) -> String {
    format!("/repos/{org}/{repo}/commits/{sha}/check-runs?per_page=100")
}

/// The discussion labels of a repo (404 = discussions or labels
/// disabled — the probe relies on this).
pub fn path_discussion_labels(org: &str, repo: &str) -> String {
    format!("/repos/{org}/{repo}/discussions/labels")
}

/// `/repos/{org}/{repo}/labels`
pub fn path_labels(org: &str, repo: &str) -> String {
    format!("/repos/{org}/{repo}/labels")
}

/// The GraphQL endpoint.
pub fn path_graphql(api: &str) -> String {
    format!("{api}/graphql")
}

// ---------- the real client ----------

/// A GitHub client over `reqwest` (REST + GraphQL).
pub struct RealGitHub {
    client: reqwest::Client,
    pub api: String,
    pub org: String,
    token: String,
}

impl RealGitHub {
    pub fn new(api: &str, org: &str, token: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api: api.trim_end_matches('/').to_string(),
            org: org.to_string(),
            token: token.to_string(),
        }
    }

    fn authed(&self) -> reqwest::header::HeaderMap {
        let mut h = reqwest::header::HeaderMap::new();
        h.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.token)
                .parse()
                .expect("ascii token"),
        );
        // GitHub rejects REST requests without a User-Agent (403
        // "administrative rules"); reqwest sends none by default.
        h.insert(
            reqwest::header::USER_AGENT,
            format!("ghsync/{}", env!("CARGO_PKG_VERSION"))
                .parse()
                .expect("ascii version"),
        );
        h.insert(
            reqwest::header::ACCEPT,
            "application/vnd.github+json".parse().expect("static"),
        );
        h.insert(
            reqwest::header::HeaderName::from_static("x-github-api-version"),
            "2022-11-28".parse().expect("static"),
        );
        h
    }

    async fn rest(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<reqwest::Response, GitHubError> {
        let url = format!("{}{}", self.api, path);
        let mut req = self.client.request(method, &url).headers(self.authed());
        if let Some(body) = body {
            req = req.json(&body);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| GitHubError::Io(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp
                .text()
                .await
                .map_err(|e| GitHubError::Io(e.to_string()))?;
            return Err(GitHubError::Http {
                status,
                url,
                message,
            });
        }
        Ok(resp)
    }

    fn issue_from(&self, repo: &str, v: &serde_json::Value) -> Issue {
        Issue {
            owner: self.org.clone(),
            repo: repo.to_string(),
            number: v.get("number").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
            title: v
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or_default()
                .to_string(),
            body: v
                .get("body")
                .and_then(|b| b.as_str())
                .unwrap_or_default()
                .to_string(),
            state: v
                .get("state")
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_string(),
            labels: v
                .get("labels")
                .and_then(|l| l.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|l| l.get("name").and_then(|n| n.as_str()).map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            node_id: v
                .get("node_id")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string(),
            milestone: v
                .pointer("/milestone/title")
                .and_then(|t| t.as_str())
                .map(str::to_string),
            url: v
                .get("url")
                .and_then(|u| u.as_str())
                .unwrap_or_default()
                .to_string(),
        }
    }
}

impl GitHub for RealGitHub {
    async fn create_issue(
        &mut self,
        repo: &str,
        title: &str,
        body: &str,
        labels: &[String],
    ) -> Result<Issue, GitHubError> {
        let payload = serde_json::json!({
            "title": title,
            "body": body,
            "labels": labels,
        });
        let resp = self
            .rest(
                reqwest::Method::POST,
                &path_repo(&self.org, repo),
                Some(payload),
            )
            .await?;
        let v = resp
            .json::<serde_json::Value>()
            .await
            .map_err(|e| GitHubError::Io(e.to_string()))?;
        Ok(self.issue_from(repo, &v))
    }

    async fn get_issue(&mut self, repo: &str, number: u32) -> Result<Issue, GitHubError> {
        let resp = self
            .rest(
                reqwest::Method::GET,
                &path_issue(&self.org, repo, number),
                None,
            )
            .await?;
        let v = resp
            .json::<serde_json::Value>()
            .await
            .map_err(|e| GitHubError::Io(e.to_string()))?;
        Ok(self.issue_from(repo, &v))
    }

    async fn issue_comments(
        &mut self,
        repo: &str,
        number: u32,
    ) -> Result<Vec<Comment>, GitHubError> {
        let mut out = Vec::new();
        for page in 1..=10u32 {
            let resp = self
                .rest(
                    reqwest::Method::GET,
                    &path_issue_comments(&self.org, repo, number, page),
                    None,
                )
                .await?;
            let arr = resp
                .json::<Vec<serde_json::Value>>()
                .await
                .map_err(|e| GitHubError::Io(e.to_string()))?;
            for c in &arr {
                out.push(Comment {
                    id: c.get("id").and_then(|i| i.as_u64()).unwrap_or(0),
                    user: c
                        .pointer("/login")
                        .and_then(|u| u.as_str())
                        .or_else(|| c.pointer("/user/login").and_then(|u| u.as_str()))
                        .unwrap_or_default()
                        .to_string(),
                    body: c
                        .get("body")
                        .and_then(|b| b.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    created_at: c
                        .get("created_at")
                        .and_then(|s| s.as_str())
                        .unwrap_or_default()
                        .to_string(),
                });
            }
            if arr.len() < 100 {
                break;
            }
        }
        Ok(out)
    }

    async fn create_milestone(
        &mut self,
        repo: &str,
        title: &str,
    ) -> Result<Milestone, GitHubError> {
        let resp = self
            .rest(
                reqwest::Method::POST,
                &format!("{}/milestones", path_repo(&self.org, repo)),
                Some(serde_json::json!({ "title": title })),
            )
            .await?;
        let v = resp
            .json::<serde_json::Value>()
            .await
            .map_err(|e| GitHubError::Io(e.to_string()))?;
        Ok(Milestone {
            number: v.get("number").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
            title: v
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or_default()
                .to_string(),
            open_issues: v.get("open_issues").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
            closed_issues: v.get("closed_issues").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
        })
    }

    async fn list_milestones(&mut self, repo: &str) -> Result<Vec<Milestone>, GitHubError> {
        let resp = self
            .rest(
                reqwest::Method::GET,
                &path_milestones(&self.org, repo),
                None,
            )
            .await?;
        let arr = resp
            .json::<Vec<serde_json::Value>>()
            .await
            .map_err(|e| GitHubError::Io(e.to_string()))?;
        Ok(arr
            .into_iter()
            .map(|v| Milestone {
                number: v.get("number").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
                title: v
                    .get("title")
                    .and_then(|t| t.as_str())
                    .unwrap_or_default()
                    .to_string(),
                open_issues: v.get("open_issues").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
                closed_issues: v.get("closed_issues").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
            })
            .collect())
    }

    async fn set_milestone(
        &mut self,
        repo: &str,
        number: u32,
        milestone: u32,
    ) -> Result<(), GitHubError> {
        self.rest(
            reqwest::Method::PATCH,
            &path_issue(&self.org, repo, number),
            Some(serde_json::json!({ "milestone": milestone })),
        )
        .await?;
        Ok(())
    }

    async fn set_labels(
        &mut self,
        repo: &str,
        number: u32,
        labels: &[String],
    ) -> Result<(), GitHubError> {
        self.rest(
            reqwest::Method::PUT,
            &format!("{}{}", path_issue(&self.org, repo, number), "/labels"),
            Some(serde_json::json!({ "labels": labels })),
        )
        .await?;
        Ok(())
    }

    async fn close_issue(&mut self, repo: &str, number: u32) -> Result<(), GitHubError> {
        self.rest(
            reqwest::Method::PATCH,
            &path_issue(&self.org, repo, number),
            Some(serde_json::json!({ "state": "closed" })),
        )
        .await?;
        Ok(())
    }

    async fn set_issue_body(
        &mut self,
        repo: &str,
        number: u32,
        body: &str,
    ) -> Result<(), GitHubError> {
        self.rest(
            reqwest::Method::PATCH,
            &path_issue(&self.org, repo, number),
            Some(serde_json::json!({ "body": body })),
        )
        .await?;
        Ok(())
    }

    async fn issues_labeled(&mut self, repo: &str, label: &str) -> Result<Vec<Issue>, GitHubError> {
        let mut out = Vec::new();
        for page in 1..=10u32 {
            let resp = self
                .rest(
                    reqwest::Method::GET,
                    &path_issues_labeled(&self.org, repo, label, page),
                    None,
                )
                .await?;
            let arr = resp
                .json::<Vec<serde_json::Value>>()
                .await
                .map_err(|e| GitHubError::Io(e.to_string()))?;
            // The issues endpoint also returns pull requests.
            for v in arr.iter().filter(|v| v.get("pull_request").is_none()) {
                out.push(self.issue_from(repo, v));
            }
            if arr.len() < 100 {
                break;
            }
        }
        out.sort_by_key(|i| i.number);
        Ok(out)
    }

    async fn comment(&mut self, repo: &str, number: u32, body: &str) -> Result<(), GitHubError> {
        self.rest(
            reqwest::Method::POST,
            &format!("{}{}", path_issue(&self.org, repo, number), "/comments"),
            Some(serde_json::json!({ "body": body })),
        )
        .await?;
        Ok(())
    }

    async fn graphql(
        &mut self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value, GitHubError> {
        let url = path_graphql(&self.api);
        let payload = serde_json::json!({ "query": query, "variables": variables });
        let resp = self
            .client
            .post(&url)
            .headers(self.authed())
            .json(&payload)
            .send()
            .await
            .map_err(|e| GitHubError::Io(e.to_string()))?;
        let status = resp.status();
        let v = resp
            .json::<serde_json::Value>()
            .await
            .map_err(|e| GitHubError::Io(e.to_string()))?;
        if let Some(errors) = v.get("errors").and_then(|e| e.as_array()) {
            let messages: Vec<&str> = errors
                .iter()
                .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                .collect();
            return Err(GitHubError::Graphql {
                url,
                message: messages.join("; "),
            });
        }
        if !status.is_success() {
            return Err(GitHubError::Http {
                status: status.as_u16(),
                url,
                message: v.to_string(),
            });
        }
        Ok(v.get("data").cloned().unwrap_or(serde_json::Value::Null))
    }

    async fn project_item_for_issue(
        &mut self,
        project_id: &str,
        repo: &str,
        number: u32,
    ) -> Result<Option<String>, GitHubError> {
        let mut after: Option<String> = None;
        for _page in 0..10 {
            let data = self
                .graphql(
                    GHSYNC_ITEMS_QUERY,
                    serde_json::json!({ "projectId": project_id, "cursor": after }),
                )
                .await?;
            let items = data
                .pointer("/node/items")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let nodes = items
                .get("nodes")
                .and_then(|n| n.as_array())
                .cloned()
                .unwrap_or_default();
            for n in nodes {
                let Some(content) = n.get("content") else {
                    continue;
                };
                let is_match = content.get("number").and_then(|v| v.as_u64())
                    == Some(u64::from(number))
                    && content.pointer("/repository/name").and_then(|v| v.as_str()) == Some(repo);
                if is_match {
                    return Ok(n.get("id").and_then(|v| v.as_str()).map(str::to_string));
                }
            }
            if !items
                .pointer("/pageInfo/hasNextPage")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                break;
            }
            after = items
                .pointer("/pageInfo/endCursor")
                .and_then(|v| v.as_str())
                .map(str::to_string);
        }
        Ok(None)
    }

    async fn add_project_item(
        &mut self,
        project_id: &str,
        node_id: &str,
    ) -> Result<String, GitHubError> {
        let data = self
            .graphql(
                GHSYNC_ADD_ITEM_MUTATION,
                serde_json::json!({ "projectId": project_id, "contentId": node_id }),
            )
            .await?;
        data.pointer("/addProjectV2ItemById/item/id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| GitHubError::Io("addProjectV2ItemById: missing item id".into()))
    }

    async fn project_field_value(
        &mut self,
        item_id: &str,
        field_id: &str,
    ) -> Result<Option<String>, GitHubError> {
        let data = self
            .graphql(
                GHSYNC_FIELD_VALUE_QUERY,
                serde_json::json!({ "itemId": item_id }),
            )
            .await?;
        let Some(nodes) = data
            .pointer("/node/fieldValues/nodes")
            .and_then(|n| n.as_array())
        else {
            return Ok(None);
        };
        // Each value node carries its field's node id (a union: the
        // matching fragment merges `field { id }` in); the item has at
        // most one value per field.
        for n in nodes {
            if n.pointer("/field/id").and_then(|v| v.as_str()) != Some(field_id) {
                continue;
            }
            return Ok(n
                .get("name")
                .and_then(|v| v.as_str())
                .or_else(|| n.get("text").and_then(|v| v.as_str()))
                .map(str::to_string));
        }
        Ok(None)
    }

    async fn set_project_field(
        &mut self,
        project_id: &str,
        item_id: &str,
        field_id: &str,
        value: &serde_json::Value,
    ) -> Result<(), GitHubError> {
        self.graphql(
            GHSYNC_SET_FIELD_MUTATION,
            serde_json::json!({
                "projectId": project_id,
                "itemId": item_id,
                "fieldId": field_id,
                "value": value
            }),
        )
        .await?;
        Ok(())
    }

    async fn get_user(&mut self) -> Result<String, GitHubError> {
        let resp = self.rest(reqwest::Method::GET, "/user", None).await?;
        let v = resp
            .json::<serde_json::Value>()
            .await
            .map_err(|e| GitHubError::Io(e.to_string()))?;
        v.get("login")
            .and_then(|l| l.as_str())
            .map(str::to_string)
            .ok_or_else(|| GitHubError::Io("GET /user: missing login".into()))
    }

    async fn list_labels(&mut self, repo: &str) -> Result<Vec<String>, GitHubError> {
        let resp = self
            .rest(reqwest::Method::GET, &path_labels(&self.org, repo), None)
            .await?;
        let v = resp
            .json::<serde_json::Value>()
            .await
            .map_err(|e| GitHubError::Io(e.to_string()))?;
        Ok(v.as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|l| l.get("name").and_then(|n| n.as_str()).map(str::to_string))
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn create_label(
        &mut self,
        repo: &str,
        name: &str,
        color: &str,
    ) -> Result<(), GitHubError> {
        self.rest(
            reqwest::Method::POST,
            &path_labels(&self.org, repo),
            Some(serde_json::json!({ "name": name, "color": color })),
        )
        .await?;
        Ok(())
    }

    async fn has_discussions(&mut self, repo: &str) -> Result<bool, GitHubError> {
        let resp = self
            .rest(reqwest::Method::GET, &path_repo(&self.org, repo), None)
            .await?;
        let v = resp
            .json::<serde_json::Value>()
            .await
            .map_err(|e| GitHubError::Io(e.to_string()))?;
        Ok(v.get("has_discussions")
            .and_then(|h| h.as_bool())
            .unwrap_or(false))
    }

    async fn get_pull(&mut self, repo: &str, number: u32) -> Result<Pull, GitHubError> {
        let resp = self
            .rest(
                reqwest::Method::GET,
                &path_pull(&self.org, repo, number),
                None,
            )
            .await?;
        let v = resp
            .json::<serde_json::Value>()
            .await
            .map_err(|e| GitHubError::Io(e.to_string()))?;
        let merged = v.get("merged").and_then(|m| m.as_bool()).unwrap_or(false);
        let head_sha = v
            .pointer("/head/sha")
            .and_then(|s| s.as_str())
            .unwrap_or_default()
            .to_string();
        let url = v
            .get("html_url")
            .and_then(|u| u.as_str())
            .unwrap_or_default()
            .to_string();
        Ok(Pull {
            number,
            merged,
            head_sha,
            url,
        })
    }

    async fn check_runs(&mut self, repo: &str, sha: &str) -> Result<Vec<CheckRun>, GitHubError> {
        let resp = self
            .rest(
                reqwest::Method::GET,
                &path_check_runs(&self.org, repo, sha),
                None,
            )
            .await?;
        let v = resp
            .json::<serde_json::Value>()
            .await
            .map_err(|e| GitHubError::Io(e.to_string()))?;
        Ok(v.get("check_runs")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| {
                        Some(CheckRun {
                            status: r.get("status")?.as_str()?.to_string(),
                            conclusion: r
                                .get("conclusion")
                                .and_then(|c| c.as_str())
                                .map(str::to_string),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn list_discussions(
        &mut self,
        repo: &str,
        category: Option<&str>,
        label: Option<&str>,
    ) -> Result<Vec<Discussion>, GitHubError> {
        let data = self
            .graphql(
                GHSYNC_DISCUSSION_LIST_QUERY,
                serde_json::json!({ "owner": self.org, "name": repo }),
            )
            .await?;
        // The query carries no category/label arguments (live-verified);
        // both filters run client-side on the selected fields.
        Ok(data
            .pointer("/repository/discussions/nodes")
            .and_then(|n| n.as_array())
            .map(|nodes| {
                nodes
                    .iter()
                    .filter_map(|d| {
                        let labels = d
                            .pointer("/labels/nodes")
                            .and_then(|l| l.as_array())
                            .map(|ls| {
                                ls.iter()
                                    .filter_map(|l| {
                                        Some(DiscussionLabel {
                                            id: l.get("id")?.as_str()?.to_string(),
                                            name: l.get("name")?.as_str()?.to_string(),
                                        })
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        let comments = d
                            .pointer("/comments/nodes")
                            .and_then(|c| c.as_array())
                            .map(|cs| {
                                cs.iter()
                                    .filter_map(|c| {
                                        c.get("body").and_then(|b| b.as_str()).map(str::to_string)
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        let category_name = d.pointer("/category/name").and_then(|c| c.as_str());
                        if let Some(want) = category
                            && category_name != Some(want)
                        {
                            return None;
                        }
                        if let Some(want) = label
                            && !labels.iter().any(|l| l.name == want)
                        {
                            return None;
                        }
                        Some(Discussion {
                            id: d.get("id")?.as_str()?.to_string(),
                            number: d.get("number")?.as_u64()? as u32,
                            title: d.get("title")?.as_str()?.to_string(),
                            body: d
                                .get("body")
                                .and_then(|b| b.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            labels,
                            comments,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn create_discussion(
        &mut self,
        repo: &str,
        category: &str,
        title: &str,
        body: &str,
    ) -> Result<Discussion, GitHubError> {
        // The category id (and repository id) come from the state query:
        // one lookup, then the create mutation.
        let data = self
            .graphql(
                GHSYNC_DISCUSSIONS_QUERY,
                serde_json::json!({ "owner": self.org, "name": repo }),
            )
            .await?;
        let url = path_graphql(&self.api);
        let repository_id = data
            .pointer("/repository/id")
            .and_then(|i| i.as_str())
            .ok_or_else(|| GitHubError::Graphql {
                url: url.clone(),
                message: format!("repository id missing for {repo}"),
            })?;
        let category_id = data
            .pointer("/repository/discussionCategories/nodes")
            .and_then(|n| n.as_array())
            .and_then(|nodes| {
                nodes
                    .iter()
                    .find(|c| c.get("name").and_then(|n| n.as_str()) == Some(category))
                    .and_then(|c| c.get("id").and_then(|i| i.as_str()))
            })
            .ok_or_else(|| GitHubError::Graphql {
                url: url.clone(),
                message: format!(
                    "discussion category `{category}` not found in {repo}; run `ghsync setup`"
                ),
            })?;
        let data = self
            .graphql(
                GHSYNC_CREATE_DISCUSSION_MUTATION,
                serde_json::json!({
                    "repositoryId": repository_id,
                    "categoryId": category_id,
                    "title": title,
                    "body": body,
                }),
            )
            .await?;
        let d = data
            .pointer("/createDiscussion/discussion")
            .ok_or_else(|| GitHubError::Graphql {
                url: url.clone(),
                message: "createDiscussion returned no discussion".into(),
            })?;
        Ok(Discussion {
            id: d
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or_default()
                .to_string(),
            number: d.get("number").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
            title: d
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or_default()
                .to_string(),
            body: body.to_string(),
            labels: Vec::new(),
            comments: Vec::new(),
        })
    }

    async fn discussion_comment(
        &mut self,
        _repo: &str,
        discussion_id: &str,
        body: &str,
    ) -> Result<(), GitHubError> {
        self.graphql(
            GHSYNC_DISCUSSION_COMMENT_MUTATION,
            serde_json::json!({
                "discussionId": discussion_id,
                "body": body,
            }),
        )
        .await?;
        Ok(())
    }

    async fn remove_discussion_label(
        &mut self,
        _repo: &str,
        discussion_id: &str,
        label_id: &str,
    ) -> Result<(), GitHubError> {
        // The label ops run through the shared labelable mutations with
        // GLOBAL node ids (REST numeric label ids are not resolvable —
        // live-verified).
        self.graphql(
            GHSYNC_REMOVE_DISCUSSION_LABEL_MUTATION,
            serde_json::json!({
                "labelableId": discussion_id,
                "labelIds": [label_id]
            }),
        )
        .await?;
        Ok(())
    }

    async fn discussion_labels_supported(&mut self, repo: &str) -> Result<bool, GitHubError> {
        // 404/410 = discussions (or labels) disabled; anything else
        // succeeds only with a real 2xx. (GitHub returns 410 Gone on the
        // discussions endpoints of repos where discussions are off —
        // live-verified.)
        let url = format!("{}{}", self.api, path_discussion_labels(&self.org, repo));
        let resp = self
            .client
            .get(&url)
            .headers(self.authed())
            .send()
            .await
            .map_err(|e| GitHubError::Io(e.to_string()))?;
        let status = resp.status();
        if matches!(status.as_u16(), 404 | 410) {
            return Ok(false);
        }
        if !status.is_success() {
            let message = resp
                .text()
                .await
                .map_err(|e| GitHubError::Io(e.to_string()))?;
            return Err(GitHubError::Http {
                status: status.as_u16(),
                url,
                message,
            });
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GitHub 403s REST calls without a User-Agent; pin the header so a
    /// regression fails CI, not the live run.
    #[test]
    fn authed_headers_carry_user_agent() {
        let gh = RealGitHub::new("https://api.github.com", "Foodshareclub", "tok");
        let h = gh.authed();
        let ua = h
            .get(reqwest::header::USER_AGENT)
            .expect("User-Agent present")
            .to_str()
            .unwrap();
        assert!(
            ua.starts_with("ghsync/"),
            "User-Agent must identify ghsync: {ua}"
        );
        assert!(h.get(reqwest::header::AUTHORIZATION).is_some());
        assert!(h.get(reqwest::header::ACCEPT).is_some());
    }

    /// The Project v2 strings are the contract with the live API; pin the
    /// load-bearing parts so any drift fails CI, not the live run.
    #[test]
    fn project_graphql_strings_are_pinned() {
        assert!(
            GHSYNC_ITEMS_QUERY
                .contains("query GhsyncProjectItems($projectId: ID!, $cursor: String)")
        );
        assert!(GHSYNC_ITEMS_QUERY.contains("items(first: 100, after: $cursor)"));
        assert!(GHSYNC_ITEMS_QUERY.contains("... on ProjectV2 {"));
        assert!(GHSYNC_ITEMS_QUERY.contains("... on Issue {"));
        assert!(GHSYNC_ITEMS_QUERY.contains("number"));
        assert!(GHSYNC_ITEMS_QUERY.contains("repository {"));
        // `Issue.nodeId` does not exist in the live schema (live-verified
        // 2026-11-13) and the item→issue match uses number + repo name,
        // so the node id is not selected.
        assert!(!GHSYNC_ITEMS_QUERY.contains("nodeId"));

        // `ProjectV2Item` has no `fieldValueByFieldId` in the live schema
        // (live-verified 2026-11-13): the value is read from the
        // `fieldValues` connection and matched on the field's node id,
        // which each value carries via the `field` selection.
        assert!(GHSYNC_FIELD_VALUE_QUERY.contains("query GhsyncFieldValue($itemId: ID!)"));
        assert!(GHSYNC_FIELD_VALUE_QUERY.contains("... on ProjectV2Item {"));
        assert!(GHSYNC_FIELD_VALUE_QUERY.contains("fieldValues(first: 100)"));
        assert!(GHSYNC_FIELD_VALUE_QUERY.contains("ProjectV2ItemFieldSingleSelectValue"));
        assert!(GHSYNC_FIELD_VALUE_QUERY.contains("ProjectV2ItemFieldTextValue"));
        assert!(GHSYNC_FIELD_VALUE_QUERY.contains("... on ProjectV2SingleSelectField {"));
        assert!(GHSYNC_FIELD_VALUE_QUERY.contains("... on ProjectV2Field {"));
        assert!(!GHSYNC_FIELD_VALUE_QUERY.contains("fieldValueByFieldId"));

        // Live-verified 2026-11-13: the value is the `ProjectV2FieldValue`
        // input object ({singleSelectOptionId | text | …}) and the input
        // also requires projectId. `ProjectV2ItemFieldItemValueUpdate` is
        // not a defined input type.
        assert!(GHSYNC_SET_FIELD_MUTATION.contains(
            "mutation GhsyncSetField($projectId: ID!, $itemId: ID!, $fieldId: ID!, $value: ProjectV2FieldValue!)"
        ));
        assert!(GHSYNC_SET_FIELD_MUTATION.contains(
            "updateProjectV2ItemFieldValue(input: {projectId: $projectId, itemId: $itemId, fieldId: $fieldId, value: $value})"
        ));
        assert!(!GHSYNC_SET_FIELD_MUTATION.contains("ProjectV2ItemFieldItemValueUpdate"));

        assert!(
            GHSYNC_ADD_ITEM_MUTATION
                .contains("mutation GhsyncAddItem($projectId: ID!, $contentId: ID!)")
        );
        assert!(GHSYNC_ADD_ITEM_MUTATION.contains(
            "addProjectV2ItemById(input: {projectId: $projectId, contentId: $contentId})"
        ));

        assert!(
            GHSYNC_LIST_PROJECTS_QUERY
                .contains("query GhsyncListProjects($org: String!, $cursor: String)")
        );
        assert!(GHSYNC_LIST_PROJECTS_QUERY.contains("projectsV2(first: 100, after: $cursor)"));
        // The org node id is selected in the same call: CreateProjectV2Input
        // requires ownerId (live-verified 2026-11-12), and there is no
        // extra API call for it.
        assert!(GHSYNC_LIST_PROJECTS_QUERY.contains(
            "organization(login: $org) {\n    id\n    projectsV2(first: 100, after: $cursor)"
        ));

        assert!(
            GHSYNC_CREATE_PROJECT_MUTATION
                .contains("mutation GhsyncCreateProject($input: CreateProjectV2Input!)")
        );
        // The payload field is `projectV2` (live-verified 2026-11-12:
        // `Field 'project' doesn't exist on type 'CreateProjectV2Payload'`).
        assert!(GHSYNC_CREATE_PROJECT_MUTATION.contains(
            "createProjectV2(input: $input) {\n    projectV2 {\n      id\n      title\n    }"
        ));

        // `fields.nodes` is the `ProjectV2FieldConfiguration` union in the
        // live schema — id/name cannot be selected directly on it
        // (live-verified 2026-11-13). Text/number fields are the
        // `ProjectV2Field` object; single-selects are
        // `ProjectV2SingleSelectField` (the old
        // `ProjectV2FieldSingleSelect` name does not exist).
        assert!(GHSYNC_FIELDS_QUERY.contains("query GhsyncProjectFields($projectId: ID!)"));
        assert!(GHSYNC_FIELDS_QUERY.contains("fields(first: 100)"));
        assert!(GHSYNC_FIELDS_QUERY.contains("... on ProjectV2Field {"));
        assert!(GHSYNC_FIELDS_QUERY.contains("... on ProjectV2SingleSelectField {"));
        assert!(GHSYNC_FIELDS_QUERY.contains("options {"));
        assert!(!GHSYNC_FIELDS_QUERY.contains("... on ProjectV2FieldSingleSelect {"));

        // Live-verified 2026-11-13: options are created WITH the field —
        // there is no separate option-create mutation in the live schema
        // — and each option input requires name, color and description.
        // The payload field is `projectV2Field` (a union: fragments only).
        assert!(GHSYNC_CREATE_SINGLE_SELECT_MUTATION.contains(
            "mutation GhsyncCreateSingleSelect($projectId: ID!, $name: String!, $options: [ProjectV2SingleSelectFieldOptionInput!]!)"
        ));
        assert!(GHSYNC_CREATE_SINGLE_SELECT_MUTATION.contains(
            "createProjectV2Field(input: {projectId: $projectId, dataType: SINGLE_SELECT, name: $name, singleSelectOptions: $options})"
        ));
        assert!(GHSYNC_CREATE_SINGLE_SELECT_MUTATION.contains("projectV2Field {"));
        assert!(
            GHSYNC_CREATE_SINGLE_SELECT_MUTATION.contains("... on ProjectV2SingleSelectField {")
        );

        assert!(
            GHSYNC_CREATE_TEXT_FIELD_MUTATION
                .contains("mutation GhsyncCreateTextField($projectId: ID!, $name: String!)")
        );
        assert!(GHSYNC_CREATE_TEXT_FIELD_MUTATION.contains(
            "createProjectV2Field(input: {projectId: $projectId, dataType: TEXT, name: $name, singleSelectOptions: []})"
        ));
        assert!(GHSYNC_CREATE_TEXT_FIELD_MUTATION.contains("projectV2Field {"));
        assert!(GHSYNC_CREATE_TEXT_FIELD_MUTATION.contains("... on ProjectV2Field {"));

        // Adding missing options to an existing field is a
        // `updateProjectV2Field` with the FULL option list (existing
        // options keep their ids); live-verified 2026-11-13.
        assert!(GHSYNC_SET_FIELD_OPTIONS_MUTATION.contains(
            "mutation GhsyncSetFieldOptions($fieldId: ID!, $options: [ProjectV2SingleSelectFieldOptionInput!]!)"
        ));
        assert!(GHSYNC_SET_FIELD_OPTIONS_MUTATION.contains(
            "updateProjectV2Field(input: {fieldId: $fieldId, singleSelectOptions: $options})"
        ));
        assert!(GHSYNC_SET_FIELD_OPTIONS_MUTATION.contains("projectV2Field {"));

        assert!(GHSYNC_VIEWS_QUERY.contains("query GhsyncProjectViews($projectId: ID!)"));
        assert!(GHSYNC_VIEWS_QUERY.contains("views(first: 100)"));

        // Live-verified 2026-11-13: the view input takes `name` (not
        // `viewName`) and a `layout` enum — the live schema has no
        // KANBAN/PHASES view types; BOARD_LAYOUT is the board grid and
        // ROADMAP_LAYOUT the closest to the old PHASES view (grouped by
        // the Phase field via visibleFieldIds, a list). The payload field
        // is `projectV2View`.
        assert!(
            GHSYNC_CREATE_KANBAN_VIEW_MUTATION
                .contains("mutation GhsyncCreateKanbanView($projectId: ID!, $name: String!)")
        );
        assert!(GHSYNC_CREATE_KANBAN_VIEW_MUTATION.contains(
            "createProjectV2View(input: {projectId: $projectId, name: $name, layout: BOARD_LAYOUT, configuration: {visibleFieldIds: []}})"
        ));
        assert!(GHSYNC_CREATE_KANBAN_VIEW_MUTATION.contains("projectV2View {"));
        assert!(
            GHSYNC_CREATE_PHASES_VIEW_MUTATION
                .contains("mutation GhsyncCreatePhasesView($projectId: ID!, $name: String!)")
        );
        assert!(GHSYNC_CREATE_PHASES_VIEW_MUTATION.contains(
            "createProjectV2View(input: {projectId: $projectId, name: $name, layout: ROADMAP_LAYOUT})"
        ));
        assert!(GHSYNC_CREATE_PHASES_VIEW_MUTATION.contains("projectV2View {"));
        assert!(!GHSYNC_CREATE_KANBAN_VIEW_MUTATION.contains("viewType"));
        assert!(!GHSYNC_CREATE_PHASES_VIEW_MUTATION.contains("viewType"));
        // Live-verified 2026-11-13: "Roadmap views do not support visible
        // fields" — the Phases view is a plain ROADMAP_LAYOUT with no
        // configuration (the grouped board grid is the Board view).
        assert!(!GHSYNC_CREATE_PHASES_VIEW_MUTATION.contains("visibleFieldIds"));

        assert!(
            GHSYNC_DISCUSSIONS_QUERY
                .contains("query GhsyncDiscussions($owner: String!, $name: String!)")
        );
        // `hasDiscussions` does not exist on Repository (live-verified);
        // the enabled flag comes from REST `GET /repos/{org}/{repo}`.
        assert!(!GHSYNC_DISCUSSIONS_QUERY.contains("hasDiscussions"));
        // The connection requires `first` (a bare selection returns a
        // null repository, live-verified).
        assert!(GHSYNC_DISCUSSIONS_QUERY.contains("discussionCategories(first: 100) {"));

        // `addDiscussionCategory` does not exist (live-verified);
        // categories are pre-existing and setup only verifies the feed
        // category among them.
        assert!(!GHSYNC_DISCUSSIONS_QUERY.contains("addDiscussionCategory"));

        assert!(
            GHSYNC_DISCUSSION_LIST_QUERY
                .contains("query GhsyncDiscussionList($owner: String!, $name: String!)")
        );
        // The `category`/`label` arguments do not exist (live-verified);
        // filtering happens client-side on the selected category/labels.
        assert!(!GHSYNC_DISCUSSION_LIST_QUERY.contains("$category"));
        assert!(!GHSYNC_DISCUSSION_LIST_QUERY.contains("$label"));
        assert!(GHSYNC_DISCUSSION_LIST_QUERY.contains("discussions(first: 100) {"));
        assert!(GHSYNC_DISCUSSION_LIST_QUERY.contains("category {"));
        assert!(GHSYNC_DISCUSSION_LIST_QUERY.contains("labels(first: 20) {"));
        assert!(GHSYNC_DISCUSSION_LIST_QUERY.contains("comments(first: 100) {"));

        assert!(GHSYNC_CREATE_DISCUSSION_MUTATION.contains(
            "mutation GhsyncCreateDiscussion($repositoryId: ID!, $categoryId: ID!, $title: String!, $body: String!)"
        ));
        // The mutation is `createDiscussion` (`addDiscussion` does not
        // exist, live-verified).
        assert!(GHSYNC_CREATE_DISCUSSION_MUTATION.contains(
            "createDiscussion(input: {repositoryId: $repositoryId, categoryId: $categoryId, title: $title, body: $body})"
        ));
        assert!(!GHSYNC_CREATE_DISCUSSION_MUTATION.contains("addDiscussion("));

        assert!(
            GHSYNC_DISCUSSION_COMMENT_MUTATION
                .contains("mutation GhsyncDiscussionComment($discussionId: ID!, $body: String!)")
        );
        assert!(
            GHSYNC_DISCUSSION_COMMENT_MUTATION.contains(
                "addDiscussionComment(input: {discussionId: $discussionId, body: $body})"
            )
        );

        // `removeDiscussionLabel` does not exist (live-verified); label
        // removal on a discussion goes through the shared
        // `removeLabelsFromLabelable` (verified live on foodshare-web).
        assert!(GHSYNC_REMOVE_DISCUSSION_LABEL_MUTATION.contains(
            "mutation GhsyncRemoveDiscussionLabel($labelableId: ID!, $labelIds: [ID!]!)"
        ));
        assert!(GHSYNC_REMOVE_DISCUSSION_LABEL_MUTATION.contains(
            "removeLabelsFromLabelable(input: {labelableId: $labelableId, labelIds: $labelIds})"
        ));
        assert!(!GHSYNC_REMOVE_DISCUSSION_LABEL_MUTATION.contains("removeDiscussionLabel"));
    }
}
