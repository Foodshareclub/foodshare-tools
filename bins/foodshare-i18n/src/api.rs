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
    #[allow(dead_code)]
    pub fn with_config(config: ClientConfig) -> Result<Self> {
        let client = FoodshareClient::with_config(config)
            .map_err(|e| anyhow::anyhow!("Failed to create client: {}", e))?;

        Ok(Self { client })
    }

    /// Get the underlying client for advanced usage
    #[must_use]
    #[allow(dead_code)]
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

        Ok((health, elapsed))
    }

    /// Get BFF info
    pub async fn bff_info(&self) -> Result<(BffInfoResponse, Duration)> {
        let (info, elapsed) = self
            .client
            .bff()
            .info_timed()
            .await
            .context("Failed to get BFF info")?;

        Ok((info, elapsed))
    }

    /// Get supported locales
    pub async fn get_locales(&self) -> Result<LocalesResponse> {
        let locales = self
            .client
            .translations()
            .locales()
            .await
            .context("Failed to fetch locales")?;

        Ok(locales)
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

        Ok((translations, elapsed))
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

        Ok((translations, elapsed))
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

        Ok(delta)
    }

    /// Audit translation coverage
    pub async fn audit_locale(&self, locale: &str, limit: usize) -> Result<AuditResponse> {
        let audit = self
            .client
            .translations()
            .audit(locale, limit)
            .await
            .context("Failed to audit locale")?;

        Ok(audit)
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

        Ok(result)
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
