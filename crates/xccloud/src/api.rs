//! The App Store Connect API client.
//!
//! Only the Xcode Cloud endpoints are wrapped: products, build runs,
//! actions, issues, artefacts and workflows.

use foodshare_core::error::{Error, Result};
use serde::de::DeserializeOwned;

use crate::auth::Credentials;
use crate::model::{
    ArtifactAttributes, BuildAttributes, IssueAttributes, ListResponse, ProductAttributes,
    SingleResponse,
};

/// The App Store Connect API root.
pub const BASE_URL: &str = "https://api.appstoreconnect.apple.com";

/// A client for the Xcode Cloud endpoints.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    token: String,
    base_url: String,
}

impl Client {
    /// Build a client, signing a fresh token for the credentials.
    pub fn new(credentials: &Credentials) -> Result<Self> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let token = crate::auth::create_token(credentials, now)
            .map_err(|e| Error::validation(format!("could not sign the token: {e}")))?;
        Self::with_token(token)
    }

    /// Build a client with a token that is already signed.
    pub fn with_token(token: String) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent("foodshare-tools/xccloud")
            .build()
            .map_err(|e| Error::process(format!("could not build the HTTP client: {e}")))?;
        Ok(Self {
            http,
            token,
            base_url: BASE_URL.to_string(),
        })
    }

    /// Point the client at a different API root.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// The CI products of the team.
    pub async fn products(&self) -> Result<ListResponse<ProductAttributes>> {
        self.get("/v1/ciProducts?include=app,bundleId").await
    }

    /// The build runs of a product, newest first.
    pub async fn build_runs(
        &self,
        product_id: &str,
        limit: usize,
    ) -> Result<ListResponse<BuildAttributes>> {
        self.get(&format!(
            "/v1/ciProducts/{product_id}/buildRuns?limit={limit}&include=workflow&sort=-number"
        ))
        .await
    }

    /// One build run.
    pub async fn build_run(&self, build_id: &str) -> Result<SingleResponse<BuildAttributes>> {
        self.get(&format!("/v1/ciBuildRuns/{build_id}")).await
    }

    /// The issues of a build run.
    pub async fn build_issues(&self, build_id: &str) -> Result<ListResponse<IssueAttributes>> {
        self.get(&format!("/v1/ciBuildRuns/{build_id}/issues"))
            .await
    }

    /// The actions of a build run.
    pub async fn actions(&self, build_id: &str) -> Result<ListResponse<BuildAttributes>> {
        self.get(&format!("/v1/ciBuildRuns/{build_id}/actions"))
            .await
    }

    /// The issues of a build action.
    pub async fn action_issues(&self, action_id: &str) -> Result<ListResponse<IssueAttributes>> {
        self.get(&format!("/v1/ciBuildActions/{action_id}/issues"))
            .await
    }

    /// The artefacts of a build action.
    pub async fn artifacts(&self, action_id: &str) -> Result<ListResponse<ArtifactAttributes>> {
        self.get(&format!("/v1/ciBuildActions/{action_id}/artifacts"))
            .await
    }

    /// The workflows of a product.
    pub async fn workflows(&self, product_id: &str) -> Result<ListResponse<serde_json::Value>> {
        self.get(&format!("/v1/ciProducts/{product_id}/workflows"))
            .await
    }

    /// Queue a build run for a workflow.
    pub async fn trigger_build(
        &self,
        workflow_id: &str,
    ) -> Result<SingleResponse<BuildAttributes>> {
        let body = serde_json::json!({
            "data": {
                "type": "ciBuildRuns",
                "relationships": {
                    "workflow": { "data": { "type": "ciWorkflows", "id": workflow_id } }
                }
            }
        });

        let url = format!("{}/v1/ciBuildRuns", self.base_url);
        let response = self
            .http
            .post(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .body(serde_json::to_vec(&body).unwrap_or_default())
            .send()
            .await
            .map_err(|e| Error::process(format!("the request failed: {e}")))?;

        Self::decode(response).await
    }

    /// The product whose name contains the given text.
    pub async fn find_product(
        &self,
        needle: &str,
    ) -> Result<Option<crate::model::Resource<ProductAttributes>>> {
        let products = self.products().await?;
        let needle = needle.to_lowercase();
        Ok(products.data.into_iter().find(|p| {
            p.attributes
                .name
                .as_deref()
                .map(|name| name.to_lowercase().contains(&needle))
                .unwrap_or(false)
        }))
    }

    /// The newest build run of a product.
    pub async fn latest_build(&self, product_id: &str) -> Result<Option<String>> {
        let runs = self.build_runs(product_id, 1).await?;
        Ok(runs.data.first().map(|run| run.id.clone()))
    }

    /// Send a GET request and decode the response.
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{path}", self.base_url);
        let response = self
            .http
            .get(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| Error::process(format!("the request failed: {e}")))?;

        Self::decode(response).await
    }

    /// Decode a response, turning an error status into an error.
    async fn decode<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| Error::process(format!("could not read the response: {e}")))?;

        if !status.is_success() {
            return Err(Error::process(format!(
                "App Store Connect API error ({}): {}",
                status.as_u16(),
                body.trim()
            )));
        }

        serde_json::from_str(&body)
            .map_err(|e| Error::process(format!("could not read the response: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::BuildAttributes;

    #[test]
    fn test_a_client_can_be_pointed_at_another_host() {
        let client = Client::with_token("token".to_string()).expect("client");
        assert_eq!(client.base_url, BASE_URL);
        let client = client.with_base_url("http://localhost:1234");
        assert_eq!(client.base_url, "http://localhost:1234");
    }

    #[test]
    fn test_the_status_of_a_build_with_nothing_set() {
        let attributes = BuildAttributes::default();
        assert_eq!(attributes.status(), "IN PROGRESS");
        assert_eq!(attributes.glyph(), "⏳");
    }
}
