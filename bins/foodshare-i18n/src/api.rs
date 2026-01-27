//! API client for translation endpoints
//!
//! This module provides a thin wrapper around `FoodshareClient` for backwards compatibility.

use crate::types::*;
use anyhow::{Context, Result};
use foodshare_api_client::{ClientConfig, FoodshareClient};
use std::time::Duration;

/// HTTP client wrapper for translation API
///
/// This is a compatibility layer that wraps `FoodshareClient` and provides
/// the same interface as the previous direct reqwest implementation.
pub struct ApiClient {
    client: FoodshareClient,
}

impl ApiClient {
    /// Create a new API client
    pub fn new() -> Result<Self> {
        let config = ClientConfig::from_env()
            .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

        let client = FoodshareClient::with_config(config)
            .map_err(|e| anyhow::anyhow!("Failed to create client: {}", e))?;

        Ok(Self { client })
    }

    /// Create a client with custom configuration
    pub fn with_config(config: ClientConfig) -> Result<Self> {
        let client = FoodshareClient::with_config(config)
            .map_err(|e| anyhow::anyhow!("Failed to create client: {}", e))?;

        Ok(Self { client })
    }

    /// Get the underlying client for advanced usage
    #[must_use]
    pub fn inner(&self) -> &FoodshareClient {
        &self.client
    }

    /// Check health of get-translations endpoint
    pub async fn health_check(&self) -> Result<(HealthResponse, Duration)> {
        let (health, elapsed) = self
            .client
            .health()
            .check_timed()
            .await
            .context("Failed to check health")?;

        // Convert from api-client types to local types
        Ok((
            HealthResponse {
                status: health.status,
                version: health.version,
                timestamp: health.timestamp,
                features: health.features.map(|f| HealthFeatures {
                    delta_sync: f.delta_sync,
                    prefetch: f.prefetch,
                }),
            },
            elapsed,
        ))
    }

    /// Get BFF info
    pub async fn bff_info(&self) -> Result<(BffInfoResponse, Duration)> {
        let (info, elapsed) = self
            .client
            .bff()
            .info_timed()
            .await
            .context("Failed to get BFF info")?;

        Ok((
            BffInfoResponse {
                success: info.success,
                service: info.service,
                version: info.version,
                endpoints: info
                    .endpoints
                    .into_iter()
                    .map(|e| BffEndpoint {
                        path: e.path,
                        method: e.method,
                        description: e.description,
                    })
                    .collect(),
            },
            elapsed,
        ))
    }

    /// Get supported locales
    pub async fn get_locales(&self) -> Result<LocalesResponse> {
        let locales = self
            .client
            .translations()
            .locales()
            .await
            .context("Failed to fetch locales")?;

        Ok(LocalesResponse {
            success: locales.success,
            locales: locales.locales,
            default: locales.default,
        })
    }

    /// Fetch translations from BFF endpoint
    pub async fn fetch_bff_translations(
        &self,
        locale: &str,
        version: Option<&str>,
    ) -> Result<(TranslationResponse, Duration)> {
        let (translations, elapsed) = self
            .client
            .bff()
            .translations_timed(locale, version)
            .await
            .context("Failed to fetch BFF translations")?;

        Ok((convert_translation_response(translations), elapsed))
    }

    /// Fetch translations from direct endpoint
    pub async fn fetch_direct_translations(
        &self,
        locale: &str,
    ) -> Result<(TranslationResponse, Duration)> {
        let (translations, elapsed) = self
            .client
            .translations()
            .get_timed(locale)
            .await
            .context("Failed to fetch direct translations")?;

        Ok((convert_translation_response(translations), elapsed))
    }

    /// Test ETag caching
    pub async fn test_etag_caching(&self, locale: &str, etag: &str) -> Result<u16> {
        self.client
            .translations()
            .test_etag_caching(locale, etag)
            .await
            .context("Failed to test ETag caching")
    }

    /// Test delta sync
    pub async fn test_delta_sync(
        &self,
        locale: &str,
        since_version: &str,
    ) -> Result<DeltaSyncResponse> {
        let delta = self
            .client
            .translations()
            .delta(locale, since_version)
            .await
            .context("Failed to test delta sync")?;

        Ok(DeltaSyncResponse {
            success: delta.success,
            has_changes: delta.has_changes,
            locale: delta.locale,
            current_version: delta.current_version,
            since_version: delta.since_version,
            delta: delta.delta.map(|d| DeltaData {
                added: d.added,
                updated: d
                    .updated
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k,
                            DeltaChange {
                                old: v.old,
                                new: v.new,
                            },
                        )
                    })
                    .collect(),
                deleted: d.deleted,
            }),
            stats: delta.stats.map(|s| DeltaStats {
                added: s.added,
                updated: s.updated,
                deleted: s.deleted,
            }),
        })
    }

    /// Audit translation coverage
    pub async fn audit_locale(&self, locale: &str, limit: usize) -> Result<AuditResponse> {
        let audit = self
            .client
            .translations()
            .audit(locale, limit)
            .await
            .context("Failed to audit locale")?;

        Ok(AuditResponse {
            success: audit.success,
            total_keys: audit.total_keys,
            untranslated_count: audit.untranslated_count,
            untranslated: audit.untranslated.map(|keys| {
                keys.into_iter()
                    .map(|k| UntranslatedKey {
                        key: k.key,
                        english_value: k.english_value,
                    })
                    .collect()
            }),
            by_category: audit.by_category,
        })
    }

    /// Translate missing keys
    pub async fn translate_batch(
        &self,
        locale: &str,
        keys: &serde_json::Value,
        apply: bool,
    ) -> Result<TranslateBatchResponse> {
        let result = self
            .client
            .translations()
            .translate_batch(locale, keys, apply)
            .await
            .context("Failed to translate batch")?;

        Ok(TranslateBatchResponse {
            success: result.success,
            translated: result.translated,
            translations: result.translations,
            new_version: result.new_version,
            message: result.message,
            error: result.error,
        })
    }

    /// Check endpoint availability
    pub async fn check_endpoint(&self, url: &str) -> Result<(u16, Duration)> {
        let status = self
            .client
            .health()
            .check_endpoint(url)
            .await
            .context("Failed to check endpoint")?;

        Ok((status.status_code, status.response_time))
    }

    /// Generate localized InfoPlist.strings files
    pub async fn generate_infoplist_strings(
        &self,
        strings: &std::collections::HashMap<String, String>,
        skip_cache: bool,
    ) -> Result<crate::types::GenerateInfoPlistStringsResponse> {
        use foodshare_api_client::endpoints::localization::GenerateInfoPlistStringsRequest;

        let request = GenerateInfoPlistStringsRequest {
            strings: strings.clone(),
            skip_cache: Some(skip_cache),
        };

        let resp = self
            .client
            .localization()
            .generate_infoplist_strings(&request)
            .await
            .context("Failed to generate InfoPlist.strings")?;

        Ok(crate::types::GenerateInfoPlistStringsResponse {
            success: resp.success,
            version: resp.version,
            locales: resp.locales,
            files: resp.files,
            lproj_folders: resp.lproj_folders,
            stats: resp.stats.map(|s| crate::types::InfoPlistStats {
                total_locales: s.total_locales,
                total_strings: s.total_strings,
                from_cache: s.from_cache,
                translated_count: s.translated_count,
                failed_count: s.failed_count,
                duration_ms: s.duration_ms,
            }),
            errors: resp.errors,
        })
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default API client")
    }
}

/// Convert from api-client TranslationResponse to local type
fn convert_translation_response(
    resp: foodshare_api_client::endpoints::translations::TranslationResponse,
) -> TranslationResponse {
    TranslationResponse {
        success: resp.success,
        data: resp.data.map(|d| TranslationData {
            messages: d.messages,
            locale: d.locale,
            version: d.version,
            updated_at: d.updated_at,
            fallback: d.fallback,
        }),
        user_context: resp.user_context.map(|c| UserContext {
            preferred_locale: c.preferred_locale,
            feature_flags: c.feature_flags,
        }),
        delta: resp.delta.map(|d| DeltaData {
            added: d.added,
            updated: d
                .updated
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        DeltaChange {
                            old: v.old,
                            new: v.new,
                        },
                    )
                })
                .collect(),
            deleted: d.deleted,
        }),
        stats: resp.stats.map(|s| DeltaStats {
            added: s.added,
            updated: s.updated,
            deleted: s.deleted,
        }),
        meta: resp.meta.map(|m| ResponseMeta {
            cached: m.cached,
            compressed: m.compressed,
            delta_sync: m.delta_sync,
            response_time_ms: m.response_time_ms,
        }),
    }
}
