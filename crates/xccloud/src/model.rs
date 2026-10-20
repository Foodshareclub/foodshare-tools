//! The App Store Connect JSON:API shapes.
//!
//! The API is JSON:API, so every resource has an `id`, a `type`,
//! `attributes` and `relationships`. The attributes differ per resource,
//! and every field is optional: the API omits what it does not have.

use serde::{Deserialize, Serialize};

/// A resource in a JSON:API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: Default + Deserialize<'de>"
))]
pub struct Resource<T> {
    /// The resource's identifier.
    pub id: String,
    /// The resource's type.
    #[serde(rename = "type")]
    pub kind: String,
    /// The resource's attributes.
    #[serde(default)]
    pub attributes: T,
    /// The resource's relationships.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationships: Option<Relationships>,
}

/// A relationship to another resource.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Relationships {
    /// The workflow a build run belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<Relationship>,
}

/// One side of a relationship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// The resource on the other side.
    #[serde(default)]
    pub data: Option<ResourceId>,
}

/// The identifier of a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceId {
    /// The identifier.
    pub id: String,
    /// The type.
    #[serde(rename = "type")]
    pub kind: String,
}

/// A list of resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: Default + Deserialize<'de>"
))]
pub struct ListResponse<T> {
    /// The resources.
    #[serde(default)]
    pub data: Vec<Resource<T>>,
    /// The resources included alongside them.
    #[serde(default)]
    pub included: Vec<Included>,
}

/// A resource included alongside the main data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Included {
    /// The identifier.
    pub id: String,
    /// The type.
    #[serde(rename = "type")]
    pub kind: String,
    /// The attributes, read loosely: the included resources differ per
    /// endpoint.
    #[serde(default)]
    pub attributes: serde_json::Value,
}

impl Included {
    /// The `name` attribute, when there is one.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.attributes.get("name").and_then(|n| n.as_str())
    }
}

/// A single resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: Default + Deserialize<'de>"
))]
pub struct SingleResponse<T> {
    /// The resource.
    pub data: Resource<T>,
}

/// The attributes of a CI product.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductAttributes {
    /// The product's name.
    #[serde(default)]
    pub name: Option<String>,
}

/// The attributes of a build run or a build action.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildAttributes {
    /// The build number.
    #[serde(default)]
    pub number: Option<i64>,
    /// The action's name.
    #[serde(default)]
    pub name: Option<String>,
    /// The type of action.
    #[serde(default)]
    pub action_type: Option<String>,
    /// How far the run has got.
    #[serde(default)]
    pub execution_progress: Option<String>,
    /// How the run ended.
    #[serde(default)]
    pub completion_status: Option<String>,
    /// When the run started.
    #[serde(default)]
    pub created_date: Option<String>,
    /// When the run started.
    #[serde(default)]
    pub started_date: Option<String>,
    /// When the run ended.
    #[serde(default)]
    pub finished_date: Option<String>,
    /// The commit the run was built from.
    #[serde(default)]
    pub source_commit: Option<SourceCommit>,
}

impl BuildAttributes {
    /// The status to show: the completion status, or the progress.
    #[must_use]
    pub fn status(&self) -> &str {
        self.completion_status
            .as_deref()
            .or(self.execution_progress.as_deref())
            .unwrap_or("IN PROGRESS")
    }

    /// The glyph that matches the status.
    #[must_use]
    pub fn glyph(&self) -> &'static str {
        match self.completion_status.as_deref() {
            Some("SUCCEEDED") => "✅",
            Some("FAILED") | Some("ERRORED") => "❌",
            Some("CANCELED") => "⏹️",
            _ => match self.execution_progress.as_deref() {
                Some("RUNNING") => "🔄",
                _ => "⏳",
            },
        }
    }
}

/// The commit a build run came from.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceCommit {
    /// The commit's SHA.
    #[serde(default)]
    pub commit_sha: Option<String>,
    /// The commit's message.
    #[serde(default)]
    pub message: Option<String>,
}

/// The attributes of an issue.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueAttributes {
    /// The kind of issue.
    #[serde(default)]
    pub issue_type: Option<String>,
    /// The issue's message.
    #[serde(default)]
    pub message: Option<String>,
    /// Where in the source the issue is.
    #[serde(default)]
    pub file_source: Option<FileSource>,
}

/// The source location of an issue.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSource {
    /// The file's path.
    #[serde(default)]
    pub path: Option<String>,
    /// The line in the file.
    #[serde(default)]
    pub line_number: Option<i64>,
}

/// The attributes of an artefact.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactAttributes {
    /// The artefact's file name.
    #[serde(default)]
    pub file_name: Option<String>,
    /// The artefact's type.
    #[serde(default)]
    pub file_type: Option<String>,
    /// Where to download it.
    #[serde(default)]
    pub download_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a_list_response_without_data_is_empty() {
        let response: ListResponse<ProductAttributes> = serde_json::from_str("{}").expect("parsed");
        assert!(response.data.is_empty());
        assert!(response.included.is_empty());
    }

    #[test]
    fn test_a_build_run_parses() {
        let json = r#"{
            "data": [{
                "id": "run-1",
                "type": "ciBuildRuns",
                "attributes": {
                    "number": 42,
                    "executionProgress": "RUNNING",
                    "sourceCommit": {"commitSha": "abcdef1234", "message": "Fix"}
                },
                "relationships": {"workflow": {"data": {"id": "wf-1", "type": "ciWorkflows"}}}
            }],
            "included": [{"id": "wf-1", "type": "ciWorkflows", "attributes": {"name": "CI"}}]
        }"#;
        let response: ListResponse<BuildAttributes> = serde_json::from_str(json).expect("parsed");
        assert_eq!(response.data.len(), 1);
        let run = &response.data[0];
        assert_eq!(run.attributes.number, Some(42));
        assert_eq!(run.attributes.status(), "RUNNING");
        assert_eq!(run.attributes.glyph(), "🔄");
        assert_eq!(
            run.relationships
                .as_ref()
                .and_then(|r| r.workflow.as_ref())
                .and_then(|w| w.data.as_ref())
                .map(|d| d.id.clone()),
            Some("wf-1".to_string())
        );
        assert_eq!(response.included[0].name(), Some("CI"));
    }

    #[test]
    fn test_a_failed_build_reports_failure() {
        let attributes = BuildAttributes {
            completion_status: Some("FAILED".to_string()),
            ..BuildAttributes::default()
        };
        assert_eq!(attributes.glyph(), "❌");
        assert_eq!(attributes.status(), "FAILED");
    }

    #[test]
    fn test_an_issue_parses() {
        let json = r#"{"data": [{"id": "i-1", "type": "ciIssues", "attributes": {
            "issueType": "CompilerError",
            "message": "cannot find 'x'",
            "fileSource": {"path": "Sources/x.swift", "lineNumber": 12}
        }}]}"#;
        let response: ListResponse<IssueAttributes> = serde_json::from_str(json).expect("parsed");
        let issue = &response.data[0].attributes;
        assert_eq!(issue.issue_type.as_deref(), Some("CompilerError"));
        assert_eq!(
            issue.file_source.as_ref().and_then(|f| f.line_number),
            Some(12)
        );
    }
}
