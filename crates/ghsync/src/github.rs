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
    /// The milestone title, if one is set.
    pub milestone: Option<String>,
    pub url: String,
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
    async fn project_field_value(
        &mut self,
        item_id: &str,
        field_id: &str,
    ) -> Result<Option<String>, GitHubError>;
    /// Set an item's field. `value` is the GraphQL field-value input,
    /// e.g. `{"singleSelectOptionId": "…"}` or `{"text": "…"}`.
    async fn set_project_field(
        &mut self,
        item_id: &str,
        field_id: &str,
        value: &serde_json::Value,
    ) -> Result<(), GitHubError>;
}

// ---------- Project v2 GraphQL (snapshot-pinned) ----------
//
// Project v2 is GraphQL-only and the strings below are the contract with
// the live API: the pinned tests in `tests` fail on any drift.

/// List a project's items, paged.
pub const GHSYNC_ITEMS_QUERY: &str = r#"query GhsyncProjectItems($projectId: ID!, $cursor: String) {
  node(id: $projectId) {
    ... on ProjectV2 {
      projectItems(first: 100, after: $cursor) {
        pageInfo {
          hasNextPage
          endCursor
        }
        nodes {
          id
          content {
            ... on Issue {
              number
              nodeId
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
pub const GHSYNC_FIELD_VALUE_QUERY: &str = r#"query GhsyncFieldValue($itemId: ID!, $fieldId: ID!) {
  node(id: $itemId) {
    ... on ProjectV2Item {
      fieldValues(first: 5, fieldId: $fieldId) {
        nodes {
          ... on ProjectV2ItemFieldSingleSelectValue {
            name
          }
          ... on ProjectV2ItemFieldTextValue {
            text
          }
        }
      }
    }
  }
}"#;

/// Set a project item field value.
pub const GHSYNC_SET_FIELD_MUTATION: &str = r#"mutation GhsyncSetField($itemId: ID!, $fieldId: ID!, $value: ProjectV2ItemFieldItemValueUpdate!) {
  updateProjectV2ItemFieldValue(input: {itemId: $itemId, fieldId: $fieldId, value: $value}) {
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
                .pointer("/node/projectItems")
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
                serde_json::json!({ "itemId": item_id, "fieldId": field_id }),
            )
            .await?;
        let Some(node) = data
            .pointer("/node/fieldValues/nodes")
            .and_then(|n| n.get(0))
        else {
            return Ok(None);
        };
        Ok(node
            .get("name")
            .and_then(|v| v.as_str())
            .or_else(|| node.get("text").and_then(|v| v.as_str()))
            .map(str::to_string))
    }

    async fn set_project_field(
        &mut self,
        item_id: &str,
        field_id: &str,
        value: &serde_json::Value,
    ) -> Result<(), GitHubError> {
        self.graphql(
            GHSYNC_SET_FIELD_MUTATION,
            serde_json::json!({ "itemId": item_id, "fieldId": field_id, "value": value }),
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Project v2 strings are the contract with the live API; pin the
    /// load-bearing parts so any drift fails CI, not the live run.
    #[test]
    fn project_graphql_strings_are_pinned() {
        assert!(
            GHSYNC_ITEMS_QUERY
                .contains("query GhsyncProjectItems($projectId: ID!, $cursor: String)")
        );
        assert!(GHSYNC_ITEMS_QUERY.contains("projectItems(first: 100, after: $cursor)"));
        assert!(GHSYNC_ITEMS_QUERY.contains("... on ProjectV2 {"));
        assert!(GHSYNC_ITEMS_QUERY.contains("... on Issue {"));
        assert!(GHSYNC_ITEMS_QUERY.contains("number"));
        assert!(GHSYNC_ITEMS_QUERY.contains("nodeId"));
        assert!(GHSYNC_ITEMS_QUERY.contains("repository {"));

        assert!(
            GHSYNC_FIELD_VALUE_QUERY
                .contains("query GhsyncFieldValue($itemId: ID!, $fieldId: ID!)")
        );
        assert!(GHSYNC_FIELD_VALUE_QUERY.contains("... on ProjectV2Item {"));
        assert!(GHSYNC_FIELD_VALUE_QUERY.contains("fieldValues(first: 5, fieldId: $fieldId)"));
        assert!(GHSYNC_FIELD_VALUE_QUERY.contains("ProjectV2ItemFieldSingleSelectValue"));
        assert!(GHSYNC_FIELD_VALUE_QUERY.contains("ProjectV2ItemFieldTextValue"));

        assert!(GHSYNC_SET_FIELD_MUTATION.contains(
            "mutation GhsyncSetField($itemId: ID!, $fieldId: ID!, $value: ProjectV2ItemFieldItemValueUpdate!)"
        ));
        assert!(GHSYNC_SET_FIELD_MUTATION.contains("updateProjectV2ItemFieldValue(input: {itemId: $itemId, fieldId: $fieldId, value: $value})"));

        assert!(
            GHSYNC_ADD_ITEM_MUTATION
                .contains("mutation GhsyncAddItem($projectId: ID!, $contentId: ID!)")
        );
        assert!(GHSYNC_ADD_ITEM_MUTATION.contains(
            "addProjectV2ItemById(input: {projectId: $projectId, contentId: $contentId})"
        ));
    }
}
