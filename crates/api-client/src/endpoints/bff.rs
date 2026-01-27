//! BFF (Backend-for-Frontend) endpoints

use crate::client::FoodshareClient;
use crate::endpoints::translations::TranslationResponse;
use crate::error::ApiResult;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// BFF API interface
#[derive(Clone)]
pub struct BffApi {
    client: FoodshareClient,
}

impl BffApi {
    /// Create a new BFF API interface
    pub(crate) fn new(client: FoodshareClient) -> Self {
        Self { client }
    }

    /// Get BFF service info
    pub async fn info(&self) -> ApiResult<BffInfoResponse> {
        self.client.get_url(self.client.bff_url()).await
    }

    /// Get BFF info with timing
    pub async fn info_timed(&self) -> ApiResult<(BffInfoResponse, Duration)> {
        self.client.timed_get_url(self.client.bff_url()).await
    }

    /// Fetch translations via BFF endpoint
    pub async fn translations(
        &self,
        locale: &str,
        version: Option<&str>,
    ) -> ApiResult<TranslationResponse> {
        let mut url = format!(
            "{}translations?locale={}&platform=ios",
            self.bff_base_url(),
            locale
        );

        if let Some(v) = version {
            url.push_str(&format!("&version={v}"));
        }

        self.client.get_url(&url).await
    }

    /// Fetch translations via BFF with timing
    pub async fn translations_timed(
        &self,
        locale: &str,
        version: Option<&str>,
    ) -> ApiResult<(TranslationResponse, Duration)> {
        let mut url = format!(
            "{}translations?locale={}&platform=ios",
            self.bff_base_url(),
            locale
        );

        if let Some(v) = version {
            url.push_str(&format!("&version={v}"));
        }

        self.client.timed_get_url(&url).await
    }

    /// Helper to get BFF base URL with trailing slash
    fn bff_base_url(&self) -> String {
        let base = self.client.bff_url();
        if base.ends_with('/') {
            base.to_string()
        } else {
            format!("{base}/")
        }
    }
}

/// BFF service info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BffInfoResponse {
    /// Success status
    pub success: bool,
    /// Service name
    pub service: String,
    /// Service version
    pub version: String,
    /// Available endpoints
    pub endpoints: Vec<BffEndpoint>,
}

/// BFF endpoint description
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BffEndpoint {
    /// Endpoint path
    pub path: String,
    /// HTTP method
    pub method: String,
    /// Description of the endpoint
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bff_info_response_deserialize() {
        let json = r#"{
            "success": true,
            "service": "foodshare-bff",
            "version": "1.0.0",
            "endpoints": [
                {
                    "path": "/translations",
                    "method": "GET",
                    "description": "Get translations for a locale"
                }
            ]
        }"#;

        let response: BffInfoResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(response.service, "foodshare-bff");
        assert_eq!(response.endpoints.len(), 1);
        assert_eq!(response.endpoints[0].path, "/translations");
    }
}
