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
        self.client
            .post("localization/translate-content", request)
            .await
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
        self.client
            .post("localization/translate-batch", request)
            .await
    }

    /// Get cached translations for content items (called by BFF)
    ///
    /// POST /localization/get-translations
    pub async fn get_cached_translations(
        &self,
        request: &GetCachedTranslationsRequest,
    ) -> ApiResult<GetCachedTranslationsResponse> {
        self.client
            .post("localization/get-translations", request)
            .await
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
        self.client
            .post("localization/ui-batch-translate", request)
            .await
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
    pub async fn backfill_posts(&self, request: &BackfillRequest) -> ApiResult<BackfillResponse> {
        self.client
            .post("localization/backfill-posts", request)
            .await
    }

    /// Backfill translations for existing challenges
    ///
    /// POST /localization/backfill-challenges
    pub async fn backfill_challenges(
        &self,
        request: &BackfillRequest,
    ) -> ApiResult<BackfillResponse> {
        self.client
            .post("localization/backfill-challenges", request)
            .await
    }

    /// Backfill translations for existing forum posts
    ///
    /// POST /localization/backfill-forum-posts
    pub async fn backfill_forum_posts(
        &self,
        request: &BackfillRequest,
    ) -> ApiResult<BackfillResponse> {
        self.client
            .post("localization/backfill-forum-posts", request)
            .await
    }

    /// Process pending translations from queue (cron job)
    ///
    /// POST /localization/process-queue
    pub async fn process_queue(&self) -> ApiResult<ProcessQueueResponse> {
        self.client
            .post("localization/process-queue", &serde_json::json!({}))
            .await
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
    /// Whether the request was successful
    pub success: bool,
    /// Service name
    pub service: String,
    /// Service version
    pub version: String,
    /// Available endpoints
    pub endpoints: Vec<EndpointInfo>,
    /// List of supported locale codes
    #[serde(rename = "supportedLocales")]
    pub supported_locales: Vec<String>,
}

/// Endpoint info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointInfo {
    /// Endpoint path
    pub path: String,
    /// HTTP method
    pub method: String,
    /// Description of the endpoint
    pub description: String,
}

/// UI strings response (simple bundle)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiStringsResponse {
    /// Whether the request was successful
    pub success: bool,
    /// Locale code
    pub locale: String,
    /// Flat map of keys to translation strings
    pub messages: serde_json::Value,
    /// Bundle version
    pub version: Option<String>,
    /// `ETag` for caching
    pub etag: Option<String>,
}

/// Translations response with delta sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationsResponse {
    /// Whether the request was successful
    pub success: bool,
    /// Full translation messages if requested
    pub data: Option<TranslationData>,
    /// User-specific localization context
    #[serde(rename = "userContext")]
    pub user_context: Option<UserContext>,
    /// Incremental changes since a version
    pub delta: Option<DeltaData>,
    /// Statistics about the delta
    pub stats: Option<DeltaStats>,
    /// Request/Response metadata
    pub meta: Option<ResponseMeta>,
}

/// Translation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationData {
    /// Flat map or nested JSON of messages
    pub messages: serde_json::Value,
    /// The locale of these messages
    pub locale: Option<String>,
    /// Version identifier for the bundle
    pub version: Option<String>,
    /// Last update timestamp
    #[serde(alias = "updated_at", alias = "updatedAt")]
    pub updated_at: Option<String>,
    /// Whether this is a fallback (e.g. English) bundle
    pub fallback: Option<bool>,
}

/// User context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// User's preferred locale code
    #[serde(rename = "preferredLocale")]
    pub preferred_locale: Option<String>,
    /// Active feature flags for this user
    #[serde(rename = "featureFlags")]
    pub feature_flags: Option<Vec<String>>,
}

/// Delta data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaData {
    /// Newly added keys
    pub added: HashMap<String, String>,
    /// Updated keys with diffs
    pub updated: HashMap<String, DeltaChange>,
    /// Keys to delete
    pub deleted: Vec<String>,
}

/// Delta change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaChange {
    /// Previous value (if known)
    pub old: Option<String>,
    /// New value
    pub new: String,
}

/// Delta stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaStats {
    /// Count of added keys
    pub added: usize,
    /// Count of updated keys
    pub updated: usize,
    /// Count of deleted keys
    pub deleted: usize,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    /// Whether the response was served from cache
    pub cached: bool,
    /// Whether the response was compressed
    pub compressed: bool,
    /// Whether delta sync was used
    #[serde(rename = "deltaSync")]
    pub delta_sync: bool,
    /// Time taken to process the request on server
    #[serde(rename = "responseTimeMs")]
    pub response_time_ms: u64,
}

/// Translate content request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateContentRequest {
    /// Content string to translate (Markdown supported)
    pub content: String,
    /// Optional source locale (auto-detected if None)
    #[serde(rename = "sourceLocale")]
    pub source_locale: Option<String>,
    /// Target locale code
    #[serde(rename = "targetLocale")]
    pub target_locale: String,
    /// Type of content for context (e.g. "`product_description`")
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
}

/// Translate content response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateContentResponse {
    /// Whether the translation was successful
    pub success: bool,
    /// The translated string
    pub translation: Option<String>,
    /// The detected source locale code
    #[serde(rename = "sourceLocale")]
    pub source_locale: Option<String>,
    /// The target locale code
    #[serde(rename = "targetLocale")]
    pub target_locale: Option<String>,
    /// Whether the translation was served from cache
    pub cached: Option<bool>,
    /// Error message if successful is false
    pub error: Option<String>,
}

/// Prewarm request
/// Prewarm request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrewarmRequest {
    /// Optional list of locales to prewarm. If None, all supported locales are prewarmed.
    pub locales: Option<Vec<String>>,
}

/// Prewarm response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrewarmResponse {
    /// Whether prewarming was successful
    pub success: bool,
    /// Response message
    pub message: Option<String>,
    /// List of locales that were prewarmed
    #[serde(rename = "localesPrewarmed")]
    pub locales_prewarmed: Option<Vec<String>>,
}

/// Translate batch request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateBatchRequest {
    /// List of items to translate
    pub items: Vec<BatchItem>,
    /// Target locales (defaults to all if None)
    #[serde(rename = "targetLocales")]
    pub target_locales: Option<Vec<String>>,
}

/// Batch item for translation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItem {
    /// Item identifier
    pub id: String,
    /// Content string
    pub content: String,
    /// Type of content
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
}

/// Translate batch response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateBatchResponse {
    /// Whether the batch request was accepted
    pub success: bool,
    /// Count of items added to the queue
    pub queued: Option<usize>,
    /// Response message
    pub message: Option<String>,
    /// Error message if successful is false
    pub error: Option<String>,
}

/// Get cached translations request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCachedTranslationsRequest {
    /// List of item IDs
    pub ids: Vec<String>,
    /// Target locale
    pub locale: String,
    /// Optional content type for context
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
}

/// Get cached translations response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCachedTranslationsResponse {
    /// Whether the check was successful
    pub success: bool,
    /// Map of found ID -> translation
    pub translations: Option<HashMap<String, String>>,
    /// List of IDs that were NOT found in cache
    pub missing: Option<Vec<String>>,
}

/// Audit response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResponse {
    /// Whether the audit was successful
    pub success: Option<bool>,
    /// Total number of keys in the system
    #[serde(rename = "totalKeys")]
    pub total_keys: Option<usize>,
    /// Number of untranslated keys
    #[serde(rename = "untranslatedCount")]
    pub untranslated_count: Option<usize>,
    /// List of untranslated key details
    pub untranslated: Option<Vec<UntranslatedKey>>,
    /// Untranslated key counts grouped by category
    #[serde(rename = "byCategory")]
    pub by_category: Option<HashMap<String, usize>>,
}

/// Untranslated key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UntranslatedKey {
    /// Translation key
    pub key: String,
    /// Original English value
    #[serde(rename = "englishValue")]
    pub english_value: Option<String>,
}

/// UI batch translate request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiBatchTranslateRequest {
    /// Locale code to translate
    pub locale: String,
    /// Optional list of specific keys
    pub keys: Option<Vec<String>>,
    /// Max items to translate in this batch
    pub limit: Option<usize>,
    /// Whether to apply translations to DB
    pub apply: Option<bool>,
}

/// UI batch translate response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiBatchTranslateResponse {
    /// Whether the batch request was successful
    pub success: bool,
    /// Count of items translated
    pub translated: Option<usize>,
    /// Map of keys to new translations
    pub translations: Option<HashMap<String, String>>,
    /// The new translation version identifier
    #[serde(rename = "newVersion")]
    pub new_version: Option<String>,
    /// Response message
    pub message: Option<String>,
    /// Error message if successful is false
    pub error: Option<String>,
}

/// Update translations request
/// Update translations request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTranslationsRequest {
    /// Locale code to update
    pub locale: String,
    /// Map of keys to translations
    pub translations: HashMap<String, String>,
}

/// Update response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResponse {
    /// Whether the update was successful
    pub success: bool,
    /// Count of updated keys
    pub updated: Option<usize>,
    /// The new translation version identifier
    #[serde(rename = "newVersion")]
    pub new_version: Option<String>,
    /// Response message
    pub message: Option<String>,
    /// Error message if successful is false
    pub error: Option<String>,
}

/// Backfill request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillRequest {
    /// Max items to process
    pub limit: Option<usize>,
    /// Whether to perform a dry run
    #[serde(rename = "dryRun")]
    pub dry_run: Option<bool>,
}

/// Backfill response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillResponse {
    /// Whether the request was successful
    pub success: bool,
    /// Items processed immediately
    pub processed: Option<usize>,
    /// Items added to background queue
    pub queued: Option<usize>,
    /// Error count
    pub errors: Option<usize>,
    /// Response message
    pub message: Option<String>,
}

/// Process queue response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessQueueResponse {
    /// Whether the queue processing run was successful
    pub success: bool,
    /// Count of items processed
    pub processed: Option<usize>,
    /// Error count
    pub errors: Option<usize>,
    /// Response message
    pub message: Option<String>,
}

/// Localization health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizationHealthResponse {
    /// Overall service status
    pub status: String,
    /// Service version
    pub version: String,
    /// Server timestamp
    pub timestamp: String,
    /// Cache subsystem health
    pub cache: Option<CacheHealth>,
    /// Database subsystem health
    pub database: Option<DatabaseHealth>,
}

/// Cache health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheHealth {
    /// Status: "ok", "degraded", or "down"
    pub status: String,
    /// Current hit rate (0.0 - 1.0)
    #[serde(rename = "hitRate")]
    pub hit_rate: Option<f64>,
    /// Count of items in cache
    pub size: Option<usize>,
}

/// Database health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealth {
    /// Status: "ok", "degraded", or "down"
    pub status: String,
    /// Connection latency in milliseconds
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
    /// Whether the request was successful
    pub success: bool,
    /// Service version
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
    /// Total number of locales processed
    #[serde(rename = "totalLocales")]
    pub total_locales: usize,
    /// Total number of strings per locale
    #[serde(rename = "totalStrings")]
    pub total_strings: usize,
    /// Number of strings served from cache
    #[serde(rename = "fromCache", default)]
    pub from_cache: usize,
    /// Number of strings translated via LLM
    #[serde(rename = "translated", alias = "translatedCount", default)]
    pub translated_count: usize,
    /// Number of strings that failed translation
    #[serde(rename = "failed", alias = "failedCount", default)]
    pub failed_count: usize,
    /// Total duration of the operation in milliseconds
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
