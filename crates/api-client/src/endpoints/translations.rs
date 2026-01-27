//! Translation endpoints

use crate::client::FoodshareClient;
use crate::error::ApiResult;
use reqwest::header::IF_NONE_MATCH;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Translation API interface
#[derive(Clone)]
pub struct TranslationsApi {
    client: FoodshareClient,
}

impl TranslationsApi {
    /// Create a new translations API interface
    pub(crate) fn new(client: FoodshareClient) -> Self {
        Self { client }
    }

    /// Get supported locales
    pub async fn locales(&self) -> ApiResult<LocalesResponse> {
        self.client.get("get-translations/locales").await
    }

    /// Get translations for a locale
    pub async fn get(&self, locale: &str) -> ApiResult<TranslationResponse> {
        let path = format!("get-translations?locale={locale}&platform=ios");
        self.client.get(&path).await
    }

    /// Get translations with timing information
    pub async fn get_timed(&self, locale: &str) -> ApiResult<(TranslationResponse, Duration)> {
        let path = format!("get-translations?locale={locale}&platform=ios");
        self.client.timed_get(&path).await
    }

    /// Get translations with ETag for caching
    ///
    /// Returns `Ok(None)` if the ETag matches (304 Not Modified).
    pub async fn get_with_etag(
        &self,
        locale: &str,
        etag: &str,
    ) -> ApiResult<Option<TranslationResponse>> {
        let path = format!("get-translations?locale={locale}&platform=ios");

        let request = self
            .client
            .request_builder(reqwest::Method::GET, &path)
            .header(IF_NONE_MATCH, format!("\"{etag}\""));

        let response = self.client.execute_raw(request).await?;
        let status = response.status();

        if status.as_u16() == 304 {
            Ok(None)
        } else if status.is_success() {
            let translations: TranslationResponse = response.json().await?;
            Ok(Some(translations))
        } else {
            let status_code = status.as_u16();
            let message = response.text().await.unwrap_or_default();
            Err(crate::error::ApiError::api_response(status_code, message))
        }
    }

    /// Check if ETag caching is working
    pub async fn test_etag_caching(&self, locale: &str, etag: &str) -> ApiResult<u16> {
        let path = format!("get-translations?locale={locale}&platform=ios");

        let request = self
            .client
            .request_builder(reqwest::Method::GET, &path)
            .header(IF_NONE_MATCH, format!("\"{etag}\""));

        let response = self.client.execute_raw(request).await?;
        Ok(response.status().as_u16())
    }

    /// Get delta sync for translations since a version
    pub async fn delta(&self, locale: &str, since_version: &str) -> ApiResult<DeltaSyncResponse> {
        let path = format!("get-translations/delta?locale={locale}&since={since_version}");
        self.client.get(&path).await
    }

    /// Audit translation coverage for a locale
    pub async fn audit(&self, locale: &str, limit: usize) -> ApiResult<AuditResponse> {
        let path = format!("translation-audit?locale={locale}&limit={limit}");
        self.client.get(&path).await
    }

    /// Translate a batch of keys
    pub async fn translate_batch(
        &self,
        locale: &str,
        keys: &serde_json::Value,
        apply: bool,
    ) -> ApiResult<TranslateBatchResponse> {
        let body = serde_json::json!({
            "locale": locale,
            "keys": keys,
            "apply": apply
        });
        self.client.post("translate-batch", &body).await
    }
}

/// Locales list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalesResponse {
    /// Success status
    pub success: bool,
    /// List of supported locale codes
    pub locales: Vec<String>,
    /// Default locale
    pub default: String,
}

/// Translation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResponse {
    /// Success status
    pub success: bool,
    /// Translation data
    pub data: Option<TranslationData>,
    /// User context information
    #[serde(rename = "userContext")]
    pub user_context: Option<UserContext>,
    /// Delta changes if requested
    pub delta: Option<DeltaData>,
    /// Delta statistics
    pub stats: Option<DeltaStats>,
    /// Response metadata
    pub meta: Option<ResponseMeta>,
}

/// Translation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationData {
    /// Translation messages as key-value pairs
    pub messages: serde_json::Value,
    /// Locale code
    pub locale: Option<String>,
    /// Version identifier
    pub version: Option<String>,
    /// Last update timestamp
    #[serde(alias = "updated_at", alias = "updatedAt")]
    pub updated_at: Option<String>,
    /// Whether this is a fallback
    pub fallback: Option<bool>,
}

/// User context from response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// User's preferred locale
    #[serde(rename = "preferredLocale")]
    pub preferred_locale: Option<String>,
    /// Active feature flags
    #[serde(rename = "featureFlags")]
    pub feature_flags: Option<Vec<String>>,
}

/// Delta changes data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaData {
    /// Added translations
    pub added: HashMap<String, String>,
    /// Updated translations
    pub updated: HashMap<String, DeltaChange>,
    /// Deleted keys
    pub deleted: Vec<String>,
}

/// A single delta change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaChange {
    /// Old value
    pub old: Option<String>,
    /// New value
    pub new: String,
}

/// Delta statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaStats {
    /// Number of added keys
    pub added: usize,
    /// Number of updated keys
    pub updated: usize,
    /// Number of deleted keys
    pub deleted: usize,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    /// Whether response was cached
    pub cached: bool,
    /// Whether response was compressed
    pub compressed: bool,
    /// Whether delta sync was used
    #[serde(rename = "deltaSync")]
    pub delta_sync: bool,
    /// Response time in milliseconds
    #[serde(rename = "responseTimeMs")]
    pub response_time_ms: u64,
}

/// Delta sync response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaSyncResponse {
    /// Success status
    pub success: bool,
    /// Whether there are changes
    #[serde(rename = "hasChanges")]
    pub has_changes: Option<bool>,
    /// Locale code
    pub locale: String,
    /// Current version
    #[serde(rename = "currentVersion")]
    pub current_version: Option<String>,
    /// Version requested since
    #[serde(rename = "sinceVersion")]
    pub since_version: Option<String>,
    /// Delta changes
    pub delta: Option<DeltaData>,
    /// Delta statistics
    pub stats: Option<DeltaStats>,
}

/// Translation audit response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResponse {
    /// Success status
    pub success: Option<bool>,
    /// Total number of keys
    #[serde(rename = "totalKeys")]
    pub total_keys: Option<usize>,
    /// Count of untranslated keys
    #[serde(rename = "untranslatedCount")]
    pub untranslated_count: Option<usize>,
    /// List of untranslated keys
    pub untranslated: Option<Vec<UntranslatedKey>>,
    /// Keys grouped by category
    #[serde(rename = "byCategory")]
    pub by_category: Option<HashMap<String, usize>>,
}

/// An untranslated key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UntranslatedKey {
    /// The translation key
    pub key: String,
    /// English value for reference
    #[serde(rename = "englishValue")]
    pub english_value: Option<String>,
}

/// Batch translation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateBatchResponse {
    /// Success status
    pub success: bool,
    /// Number of translations completed
    pub translated: Option<usize>,
    /// Translated key-value pairs
    pub translations: Option<HashMap<String, String>>,
    /// New version after translation
    #[serde(rename = "newVersion")]
    pub new_version: Option<String>,
    /// Success message
    pub message: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locales_response_deserialize() {
        let json = r#"{
            "success": true,
            "locales": ["en", "de", "fr"],
            "default": "en"
        }"#;

        let response: LocalesResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(response.locales.len(), 3);
        assert_eq!(response.default, "en");
    }

    #[test]
    fn test_translation_response_deserialize() {
        let json = r#"{
            "success": true,
            "data": {
                "messages": {"hello": "Hallo"},
                "locale": "de",
                "version": "v1.0.0"
            }
        }"#;

        let response: TranslationResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert!(response.data.is_some());
        assert_eq!(response.data.as_ref().unwrap().locale, Some("de".to_string()));
    }

    #[test]
    fn test_delta_sync_response_deserialize() {
        let json = r#"{
            "success": true,
            "hasChanges": true,
            "locale": "de",
            "currentVersion": "v1.0.1",
            "sinceVersion": "v1.0.0",
            "stats": {
                "added": 5,
                "updated": 2,
                "deleted": 1
            }
        }"#;

        let response: DeltaSyncResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(response.has_changes, Some(true));
        assert!(response.stats.is_some());
        let stats = response.stats.unwrap();
        assert_eq!(stats.added, 5);
    }
}
