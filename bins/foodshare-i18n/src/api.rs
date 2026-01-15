//! API client for translation endpoints

use crate::config::{BASE_URL, BFF_URL};
use crate::types::*;
use anyhow::{Context, Result};
use reqwest::Client;
use std::time::{Duration, Instant};

/// HTTP client wrapper for translation API
pub struct ApiClient {
    client: Client,
}

impl ApiClient {
    /// Create a new API client
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client })
    }

    /// Check health of get-translations endpoint
    pub async fn health_check(&self) -> Result<(HealthResponse, Duration)> {
        let start = Instant::now();
        let url = format!("{}/get-translations/health", BASE_URL);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to health endpoint")?;

        let elapsed = start.elapsed();
        let health: HealthResponse = response
            .json()
            .await
            .context("Failed to parse health response")?;

        Ok((health, elapsed))
    }

    /// Get BFF info
    pub async fn bff_info(&self) -> Result<(BffInfoResponse, Duration)> {
        let start = Instant::now();

        let response = self
            .client
            .get(BFF_URL)
            .send()
            .await
            .context("Failed to connect to BFF endpoint")?;

        let elapsed = start.elapsed();
        let info: BffInfoResponse = response
            .json()
            .await
            .context("Failed to parse BFF response")?;

        Ok((info, elapsed))
    }

    /// Get supported locales
    pub async fn get_locales(&self) -> Result<LocalesResponse> {
        let url = format!("{}/get-translations/locales", BASE_URL);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch locales")?;

        response
            .json()
            .await
            .context("Failed to parse locales response")
    }

    /// Fetch translations from BFF endpoint
    pub async fn fetch_bff_translations(
        &self,
        locale: &str,
        version: Option<&str>,
    ) -> Result<(TranslationResponse, Duration)> {
        let start = Instant::now();
        let mut url = format!("{}/translations?locale={}&platform=ios", BFF_URL, locale);

        if let Some(v) = version {
            url.push_str(&format!("&version={}", v));
        }

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch BFF translations")?;

        let elapsed = start.elapsed();
        let translations: TranslationResponse = response
            .json()
            .await
            .context("Failed to parse BFF translation response")?;

        Ok((translations, elapsed))
    }

    /// Fetch translations from direct endpoint
    pub async fn fetch_direct_translations(
        &self,
        locale: &str,
    ) -> Result<(TranslationResponse, Duration)> {
        let start = Instant::now();
        let url = format!(
            "{}/get-translations?locale={}&platform=ios",
            BASE_URL, locale
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch direct translations")?;

        let elapsed = start.elapsed();
        let translations: TranslationResponse = response
            .json()
            .await
            .context("Failed to parse direct translation response")?;

        Ok((translations, elapsed))
    }

    /// Test ETag caching
    pub async fn test_etag_caching(&self, locale: &str, etag: &str) -> Result<u16> {
        let url = format!(
            "{}/get-translations?locale={}&platform=ios",
            BASE_URL, locale
        );

        let response = self
            .client
            .get(&url)
            .header("If-None-Match", format!("\"{}\"", etag))
            .send()
            .await
            .context("Failed to test ETag caching")?;

        Ok(response.status().as_u16())
    }

    /// Test delta sync
    pub async fn test_delta_sync(
        &self,
        locale: &str,
        since_version: &str,
    ) -> Result<DeltaSyncResponse> {
        let url = format!(
            "{}/get-translations/delta?locale={}&since={}",
            BASE_URL, locale, since_version
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to test delta sync")?;

        response
            .json()
            .await
            .context("Failed to parse delta sync response")
    }

    /// Audit translation coverage
    pub async fn audit_locale(&self, locale: &str, limit: usize) -> Result<AuditResponse> {
        let url = format!(
            "{}/translation-audit?locale={}&limit={}",
            BASE_URL, locale, limit
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to audit locale")?;

        response
            .json()
            .await
            .context("Failed to parse audit response")
    }

    /// Translate missing keys
    pub async fn translate_batch(
        &self,
        locale: &str,
        keys: &serde_json::Value,
        apply: bool,
    ) -> Result<TranslateBatchResponse> {
        let url = format!("{}/translate-batch", BASE_URL);

        let body = serde_json::json!({
            "locale": locale,
            "keys": keys,
            "apply": apply
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to translate batch")?;

        response
            .json()
            .await
            .context("Failed to parse translate response")
    }

    /// Check endpoint availability
    pub async fn check_endpoint(&self, url: &str) -> Result<(u16, Duration)> {
        let start = Instant::now();

        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to check endpoint")?;

        Ok((response.status().as_u16(), start.elapsed()))
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default API client")
    }
}
