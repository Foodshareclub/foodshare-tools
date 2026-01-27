//! Products API endpoints
//!
//! Maps to the `/api-v1-products` Edge Function which provides:
//! - List products with filters and pagination
//! - Get single product by ID
//! - Create new product/listing
//! - Update existing product
//! - Delete product

use crate::client::FoodshareClient;
use crate::error::ApiResult;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Products API interface
///
/// This maps to the `/api-v1-products` Edge Function in foodshare-backend.
#[derive(Clone)]
pub struct ProductsApi {
    client: FoodshareClient,
}

impl ProductsApi {
    /// Create a new products API interface
    pub(crate) fn new(client: FoodshareClient) -> Self {
        Self { client }
    }

    /// List products with filters and pagination
    ///
    /// GET /api-v1-products
    pub async fn list(&self, params: &ListProductsParams) -> ApiResult<ListProductsResponse> {
        let mut path = "api-v1-products?".to_string();
        let mut query_parts = Vec::new();

        if let Some(ref post_type) = params.post_type {
            query_parts.push(format!("postType={post_type}"));
        }
        if let Some(category_id) = params.category_id {
            query_parts.push(format!("categoryId={category_id}"));
        }
        if let Some(lat) = params.lat {
            query_parts.push(format!("lat={lat}"));
        }
        if let Some(lng) = params.lng {
            query_parts.push(format!("lng={lng}"));
        }
        if let Some(radius) = params.radius {
            query_parts.push(format!("radius={radius}"));
        }
        if let Some(ref cursor) = params.cursor {
            query_parts.push(format!("cursor={cursor}"));
        }
        if let Some(limit) = params.limit {
            query_parts.push(format!("limit={limit}"));
        }
        if let Some(ref user_id) = params.user_id {
            query_parts.push(format!("userId={user_id}"));
        }

        path.push_str(&query_parts.join("&"));
        self.client.get(&path).await
    }

    /// List products with timing
    pub async fn list_timed(
        &self,
        params: &ListProductsParams,
    ) -> ApiResult<(ListProductsResponse, Duration)> {
        let mut path = "api-v1-products?".to_string();
        let mut query_parts = Vec::new();

        if let Some(ref post_type) = params.post_type {
            query_parts.push(format!("postType={post_type}"));
        }
        if let Some(limit) = params.limit {
            query_parts.push(format!("limit={limit}"));
        }

        path.push_str(&query_parts.join("&"));
        self.client.timed_get(&path).await
    }

    /// Get a single product by ID
    ///
    /// GET /api-v1-products?id=<id>
    pub async fn get(&self, id: &str) -> ApiResult<GetProductResponse> {
        let path = format!("api-v1-products?id={id}");
        self.client.get(&path).await
    }

    /// Create a new product
    ///
    /// POST /api-v1-products
    pub async fn create(&self, product: &CreateProductRequest) -> ApiResult<CreateProductResponse> {
        self.client.post("api-v1-products", product).await
    }

    /// Update an existing product
    ///
    /// PUT /api-v1-products?id=<id>
    pub async fn update(
        &self,
        id: &str,
        product: &UpdateProductRequest,
    ) -> ApiResult<UpdateProductResponse> {
        // Note: PUT is not directly supported by our client, so we use POST with method override
        // In practice, the backend may accept POST for updates as well
        let path = format!("api-v1-products?id={id}");
        self.client.post(&path, product).await
    }

    /// Delete a product
    ///
    /// DELETE /api-v1-products?id=<id>
    pub async fn delete(&self, id: &str) -> ApiResult<DeleteProductResponse> {
        let path = format!("api-v1-products?id={id}");
        self.client.post(&path, &serde_json::json!({"_method": "DELETE"})).await
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Parameters for listing products
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListProductsParams {
    /// Filter by post type: "food", "non-food", "request"
    pub post_type: Option<String>,
    /// Filter by category ID
    pub category_id: Option<i64>,
    /// Latitude for geo-search
    pub lat: Option<f64>,
    /// Longitude for geo-search
    pub lng: Option<f64>,
    /// Radius in km for geo-search
    pub radius: Option<f64>,
    /// Cursor for pagination
    pub cursor: Option<String>,
    /// Page size limit (max 50)
    pub limit: Option<u32>,
    /// Filter by user ID
    pub user_id: Option<String>,
}

impl ListProductsParams {
    /// Create new params with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by post type
    pub fn with_post_type(mut self, post_type: impl Into<String>) -> Self {
        self.post_type = Some(post_type.into());
        self
    }

    /// Filter by location
    pub fn with_location(mut self, lat: f64, lng: f64, radius: f64) -> Self {
        self.lat = Some(lat);
        self.lng = Some(lng);
        self.radius = Some(radius);
        self
    }

    /// Set pagination cursor
    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }

    /// Set page size
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Filter by user
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
}

/// List products response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProductsResponse {
    pub success: bool,
    pub data: Option<Vec<Product>>,
    pub pagination: Option<PaginationInfo>,
    pub error: Option<ErrorInfo>,
}

/// Single product response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProductResponse {
    pub success: bool,
    pub data: Option<Product>,
    pub error: Option<ErrorInfo>,
}

/// Create product request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductRequest {
    pub title: String,
    pub description: Option<String>,
    pub images: Vec<String>,
    #[serde(rename = "postType")]
    pub post_type: String,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(rename = "pickupAddress")]
    pub pickup_address: Option<String>,
    #[serde(rename = "pickupTime")]
    pub pickup_time: Option<String>,
    #[serde(rename = "categoryId")]
    pub category_id: Option<i64>,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<String>,
}

/// Create product response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductResponse {
    pub success: bool,
    pub data: Option<Product>,
    pub error: Option<ErrorInfo>,
}

/// Update product request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProductRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub images: Option<Vec<String>>,
    #[serde(rename = "pickupAddress")]
    pub pickup_address: Option<String>,
    #[serde(rename = "pickupTime")]
    pub pickup_time: Option<String>,
    #[serde(rename = "categoryId")]
    pub category_id: Option<i64>,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<String>,
    #[serde(rename = "isActive")]
    pub is_active: Option<bool>,
    /// Required for optimistic locking
    pub version: i64,
}

/// Update product response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProductResponse {
    pub success: bool,
    pub data: Option<Product>,
    pub error: Option<ErrorInfo>,
}

/// Delete product response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteProductResponse {
    pub success: bool,
    pub error: Option<ErrorInfo>,
}

/// Product entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub images: Vec<String>,
    #[serde(rename = "postType")]
    pub post_type: String,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(rename = "pickupAddress")]
    pub pickup_address: Option<String>,
    #[serde(rename = "pickupTime")]
    pub pickup_time: Option<String>,
    #[serde(rename = "categoryId")]
    pub category_id: Option<i64>,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<String>,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
    pub version: i64,
    /// Distance in km (when geo-search is used)
    pub distance: Option<f64>,
    /// Owner profile (when included)
    pub owner: Option<ProductOwner>,
}

/// Product owner info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductOwner {
    pub id: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub rating: Option<f64>,
}

/// Pagination info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub cursor: Option<String>,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
    pub total: Option<i64>,
}

/// Error info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_params_builder() {
        let params = ListProductsParams::new()
            .with_post_type("food")
            .with_location(52.52, 13.405, 10.0)
            .with_limit(20);

        assert_eq!(params.post_type, Some("food".to_string()));
        assert_eq!(params.lat, Some(52.52));
        assert_eq!(params.lng, Some(13.405));
        assert_eq!(params.radius, Some(10.0));
        assert_eq!(params.limit, Some(20));
    }

    #[test]
    fn test_product_deserialize() {
        let json = r#"{
            "id": "123",
            "title": "Fresh Bread",
            "description": "Homemade bread",
            "images": ["https://example.com/img.jpg"],
            "postType": "food",
            "latitude": 52.52,
            "longitude": 13.405,
            "isActive": true,
            "createdAt": "2024-01-01T00:00:00Z",
            "version": 1
        }"#;

        let product: Product = serde_json::from_str(json).unwrap();
        assert_eq!(product.id, "123");
        assert_eq!(product.title, "Fresh Bread");
        assert_eq!(product.post_type, "food");
    }
}
