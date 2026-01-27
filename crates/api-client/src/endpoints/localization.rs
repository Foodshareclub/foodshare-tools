//! Localization endpoints (consolidated translation service)
//!
//! Maps to the `/localization` Edge Function which provides:
//! - UI string bundles
//! - Delta sync for translations
//! - Dynamic content translation via LLM
//! - Batch translation operations
//! - Translation auditing

use crate::client::FoodshareClient;
use crate::error::ApiResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Localization API interface
///
/// This maps to the consolidated `/localization` Edge Function in foodshare-backend.
#[derive(Clone)]
pub struct LocalizationApi {
    client: FoodshareClient,
}

impl LocalizationApi {
    /// Create a new localization API interface
    pub(crate) fn new(client: FoodshareClient) -> Self {
        Self { client }
    }

    /// Get service info
    pub async fn info(&self) -> ApiResult<LocalizationServiceInfo> {
        self.client.get("localization").await
    }

    /// Get UI string bundles (simple, fast)
    ///
    /// GET /localization?locale=<locale>
    pub async fn ui_strings(&self, locale: &str) -> ApiResult<UiStringsResponse> {
        let path = format!("localization?locale={locale}");
        self.client.get(&path).await
    }

    /// Get UI strings with delta sync and user context
    ///
    /// GET /localization/translations?locale=<locale>&platform=ios
    pub async fn translations(
        &self,
        locale: &str,
        platform: &str,
    ) -> ApiResult<TranslationsResponse> {
        let path = format!("localization/translations?locale={locale}&platform={platform}");
        self.client.get(&path).await
    }

    /// Get translations with timing
    pub async fn translations_timed(
        &self,
        locale: &str,
        platform: &str,
    ) -> ApiResult<(TranslationsResponse, Duration)> {
        let path = format!("localization/translations?locale={locale}&platform={platform}");
        self.client.timed_get(&path).await
    }

    /// Translate dynamic content via self-hosted LLM
    ///
    /// POST /localization/translate-content
    pub async fn translate_content(
        &self,
        request: &TranslateContentRequest,
    ) -> ApiResult<TranslateContentResponse> {
        self.client.post("localization/translate-content", request).await
    }

    /// Prewarm translation cache (fire-and-forget)
    ///
    /// POST /localization/prewarm
    pub async fn prewarm(&self, request: &PrewarmRequest) -> ApiResult<PrewarmResponse> {
        self.client.post("localization/prewarm", request).await
    }

    /// Batch translate content to all locales (background)
    ///
    /// POST /localization/translate-batch
    pub async fn translate_batch(
        &self,
        request: &TranslateBatchRequest,
    ) -> ApiResult<TranslateBatchResponse> {
        self.client.post("localization/translate-batch", request).await
    }

    /// Get cached translations for content items (called by BFF)
    ///
    /// POST /localization/get-translations
    pub async fn get_cached_translations(
        &self,
        request: &GetCachedTranslationsRequest,
    ) -> ApiResult<GetCachedTranslationsResponse> {
        self.client.post("localization/get-translations", request).await
    }

    /// Audit untranslated UI strings
    ///
    /// GET /localization/audit?locale=<locale>&limit=<limit>
    pub async fn audit(&self, locale: &str, limit: usize) -> ApiResult<AuditResponse> {
        let path = format!("localization/audit?locale={locale}&limit={limit}");
        self.client.get(&path).await
    }

    /// Batch translate UI strings with self-hosted LLM
    ///
    /// POST /localization/ui-batch-translate
    pub async fn ui_batch_translate(
        &self,
        request: &UiBatchTranslateRequest,
    ) -> ApiResult<UiBatchTranslateResponse> {
        self.client.post("localization/ui-batch-translate", request).await
    }

    /// Update UI string translations
    ///
    /// POST /localization/update
    pub async fn update(&self, request: &UpdateTranslationsRequest) -> ApiResult<UpdateResponse> {
        self.client.post("localization/update", request).await
    }

    /// Backfill translations for existing posts
    ///
    /// POST /localization/backfill-posts
    pub async fn backfill_posts(
        &self,
        request: &BackfillRequest,
    ) -> ApiResult<BackfillResponse> {
        self.client.post("localization/backfill-posts", request).await
    }

    /// Backfill translations for existing challenges
    ///
    /// POST /localization/backfill-challenges
    pub async fn backfill_challenges(
        &self,
        request: &BackfillRequest,
    ) -> ApiResult<BackfillResponse> {
        self.client.post("localization/backfill-challenges", request).await
    }

    /// Backfill translations for existing forum posts
    ///
    /// POST /localization/backfill-forum-posts
    pub async fn backfill_forum_posts(
        &self,
        request: &BackfillRequest,
    ) -> ApiResult<BackfillResponse> {
        self.client.post("localization/backfill-forum-posts", request).await
    }

    /// Process pending translations from queue (cron job)
    ///
    /// POST /localization/process-queue
    pub async fn process_queue(&self) -> ApiResult<ProcessQueueResponse> {
        self.client.post("localization/process-queue", &serde_json::json!({})).await
    }

    /// Health check
    ///
    /// GET /localization/health
    pub async fn health(&self) -> ApiResult<LocalizationHealthResponse> {
        self.client.get("localization/health").await
    }

    /// Generate localized InfoPlist.strings files
    ///
    /// POST /localization/generate-infoplist-strings
    pub async fn generate_infoplist_strings(
        &self,
        request: &GenerateInfoPlistStringsRequest,
    ) -> ApiResult<GenerateInfoPlistStringsResponse> {
        self.client
            .post("localization/generate-infoplist-strings", request)
            .await
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Service info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizationServiceInfo {
    pub success: bool,
    pub service: String,
    pub version: String,
    pub endpoints: Vec<EndpointInfo>,
    #[serde(rename = "supportedLocales")]
    pub supported_locales: Vec<String>,
}

/// Endpoint info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointInfo {
    pub path: String,
    pub method: String,
    pub description: String,
}

/// UI strings response (simple bundle)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiStringsResponse {
    pub success: bool,
    pub locale: String,
    pub messages: serde_json::Value,
    pub version: Option<String>,
    pub etag: Option<String>,
}

/// Translations response with delta sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationsResponse {
    pub success: bool,
    pub data: Option<TranslationData>,
    #[serde(rename = "userContext")]
    pub user_context: Option<UserContext>,
    pub delta: Option<DeltaData>,
    pub stats: Option<DeltaStats>,
    pub meta: Option<ResponseMeta>,
}

/// Translation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationData {
    pub messages: serde_json::Value,
    pub locale: Option<String>,
    pub version: Option<String>,
    #[serde(alias = "updated_at", alias = "updatedAt")]
    pub updated_at: Option<String>,
    pub fallback: Option<bool>,
}

/// User context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    #[serde(rename = "preferredLocale")]
    pub preferred_locale: Option<String>,
    #[serde(rename = "featureFlags")]
    pub feature_flags: Option<Vec<String>>,
}

/// Delta data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaData {
    pub added: HashMap<String, String>,
    pub updated: HashMap<String, DeltaChange>,
    pub deleted: Vec<String>,
}

/// Delta change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaChange {
    pub old: Option<String>,
    pub new: String,
}

/// Delta stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaStats {
    pub added: usize,
    pub updated: usize,
    pub deleted: usize,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    pub cached: bool,
    pub compressed: bool,
    #[serde(rename = "deltaSync")]
    pub delta_sync: bool,
    #[serde(rename = "responseTimeMs")]
    pub response_time_ms: u64,
}

/// Translate content request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateContentRequest {
    pub content: String,
    #[serde(rename = "sourceLocale")]
    pub source_locale: Option<String>,
    #[serde(rename = "targetLocale")]
    pub target_locale: String,
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
}

/// Translate content response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateContentResponse {
    pub success: bool,
    pub translation: Option<String>,
    #[serde(rename = "sourceLocale")]
    pub source_locale: Option<String>,
    #[serde(rename = "targetLocale")]
    pub target_locale: Option<String>,
    pub cached: Option<bool>,
    pub error: Option<String>,
}

/// Prewarm request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrewarmRequest {
    pub locales: Option<Vec<String>>,
}

/// Prewarm response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrewarmResponse {
    pub success: bool,
    pub message: Option<String>,
    #[serde(rename = "localesPrewarmed")]
    pub locales_prewarmed: Option<Vec<String>>,
}

/// Translate batch request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateBatchRequest {
    pub items: Vec<BatchItem>,
    #[serde(rename = "targetLocales")]
    pub target_locales: Option<Vec<String>>,
}

/// Batch item for translation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItem {
    pub id: String,
    pub content: String,
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
}

/// Translate batch response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateBatchResponse {
    pub success: bool,
    pub queued: Option<usize>,
    pub message: Option<String>,
    pub error: Option<String>,
}

/// Get cached translations request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCachedTranslationsRequest {
    pub ids: Vec<String>,
    pub locale: String,
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
}

/// Get cached translations response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCachedTranslationsResponse {
    pub success: bool,
    pub translations: Option<HashMap<String, String>>,
    pub missing: Option<Vec<String>>,
}

/// Audit response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResponse {
    pub success: Option<bool>,
    #[serde(rename = "totalKeys")]
    pub total_keys: Option<usize>,
    #[serde(rename = "untranslatedCount")]
    pub untranslated_count: Option<usize>,
    pub untranslated: Option<Vec<UntranslatedKey>>,
    #[serde(rename = "byCategory")]
    pub by_category: Option<HashMap<String, usize>>,
}

/// Untranslated key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UntranslatedKey {
    pub key: String,
    #[serde(rename = "englishValue")]
    pub english_value: Option<String>,
}

/// UI batch translate request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiBatchTranslateRequest {
    pub locale: String,
    pub keys: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub apply: Option<bool>,
}

/// UI batch translate response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiBatchTranslateResponse {
    pub success: bool,
    pub translated: Option<usize>,
    pub translations: Option<HashMap<String, String>>,
    #[serde(rename = "newVersion")]
    pub new_version: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
}

/// Update translations request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTranslationsRequest {
    pub locale: String,
    pub translations: HashMap<String, String>,
}

/// Update response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResponse {
    pub success: bool,
    pub updated: Option<usize>,
    #[serde(rename = "newVersion")]
    pub new_version: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
}

/// Backfill request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillRequest {
    pub limit: Option<usize>,
    #[serde(rename = "dryRun")]
    pub dry_run: Option<bool>,
}

/// Backfill response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillResponse {
    pub success: bool,
    pub processed: Option<usize>,
    pub queued: Option<usize>,
    pub errors: Option<usize>,
    pub message: Option<String>,
}

/// Process queue response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessQueueResponse {
    pub success: bool,
    pub processed: Option<usize>,
    pub errors: Option<usize>,
    pub message: Option<String>,
}

/// Localization health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizationHealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: String,
    pub cache: Option<CacheHealth>,
    pub database: Option<DatabaseHealth>,
}

/// Cache health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheHealth {
    pub status: String,
    #[serde(rename = "hitRate")]
    pub hit_rate: Option<f64>,
    pub size: Option<usize>,
}

/// Database health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub status: String,
    #[serde(rename = "latencyMs")]
    pub latency_ms: Option<u64>,
}

/// Generate InfoPlist.strings request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateInfoPlistStringsRequest {
    /// Permission strings to translate (key -> English value)
    pub strings: HashMap<String, String>,
    /// Skip cache and force fresh translations
    #[serde(rename = "skipCache", skip_serializing_if = "Option::is_none")]
    pub skip_cache: Option<bool>,
}

/// Generate InfoPlist.strings response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateInfoPlistStringsResponse {
    pub success: bool,
    pub version: Option<String>,
    /// Translations by locale
    pub locales: HashMap<String, HashMap<String, String>>,
    /// Formatted InfoPlist.strings file content by locale
    pub files: HashMap<String, String>,
    /// Mapping of locale code to .lproj folder name
    #[serde(rename = "lprojFolders")]
    pub lproj_folders: HashMap<String, String>,
    /// Generation statistics
    pub stats: Option<InfoPlistStats>,
    /// Any errors or warnings during generation
    #[serde(default)]
    pub errors: Vec<String>,
}

/// InfoPlist.strings generation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoPlistStats {
    #[serde(rename = "totalLocales")]
    pub total_locales: usize,
    #[serde(rename = "totalStrings")]
    pub total_strings: usize,
    #[serde(rename = "fromCache", default)]
    pub from_cache: usize,
    #[serde(rename = "translated", alias = "translatedCount", default)]
    pub translated_count: usize,
    #[serde(rename = "failed", alias = "failedCount", default)]
    pub failed_count: usize,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_info_deserialize() {
        let json = r#"{
            "success": true,
            "service": "localization",
            "version": "2.1.0",
            "endpoints": [
                { "path": "/localization", "method": "GET", "description": "UI strings" }
            ],
            "supportedLocales": ["en", "de", "fr"]
        }"#;

        let info: LocalizationServiceInfo = serde_json::from_str(json).unwrap();
        assert!(info.success);
        assert_eq!(info.service, "localization");
        assert_eq!(info.supported_locales.len(), 3);
    }
}
