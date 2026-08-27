//! Vector search and semantic search endpoint definitions.

use crate::client::FoodshareClient;
use crate::error::ApiResult;
use serde::{Deserialize, Serialize};

/// Vector search API namespace.
pub struct SearchApi<'a> {
    /// Reference to the underlying client
    pub client: &'a FoodshareClient,
}

/// Request parameters for vector and semantic search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchRequest {
    /// Text search query
    pub query: String,
    /// Minimum cosine similarity threshold (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_threshold: Option<f32>,
    /// Max number of results to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_count: Option<usize>,
    /// Optional latitude for geo-distance ranking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    /// Optional longitude for geo-distance ranking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    /// Max radius in kilometers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius_km: Option<f64>,
    /// Filter by item category (Food, Things, Borrow)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

/// Individual search result match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    /// Item UUID or ID
    pub id: String,
    /// Item title
    pub title: String,
    /// Item description
    pub description: Option<String>,
    /// Cosine similarity score (0.0 to 1.0)
    pub similarity: f32,
    /// Distance in kilometers if coordinate provided
    pub distance_km: Option<f64>,
}

/// Search response from `/api-v1-vector-search`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResponse {
    /// Success indicator
    pub success: bool,
    /// AI embedding provider used (supabase_ai_gte_small, z_ai, huggingface)
    pub provider: Option<String>,
    /// Server processing latency in milliseconds
    pub latency_ms: Option<u64>,
    /// Matched listings
    pub data: Vec<SearchMatch>,
}

impl<'a> SearchApi<'a> {
    /// Execute a semantic vector search query against the backend.
    pub async fn vector_search(&self, request: &VectorSearchRequest) -> ApiResult<VectorSearchResponse> {
        self.client.post("api-v1-vector-search", request).await
    }
}
