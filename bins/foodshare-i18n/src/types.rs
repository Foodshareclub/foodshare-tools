//! Type definitions for API responses

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Health check response
#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: String,
    pub features: Option<HealthFeatures>,
}

#[derive(Debug, Deserialize)]
pub struct HealthFeatures {
    #[serde(rename = "deltaSync")]
    pub delta_sync: Option<bool>,
    pub prefetch: Option<bool>,
}

/// Locales list response
#[derive(Debug, Deserialize)]
pub struct LocalesResponse {
    pub success: bool,
    pub locales: Vec<String>,
    pub default: String,
}

/// BFF info response
#[derive(Debug, Deserialize)]
pub struct BffInfoResponse {
    pub success: bool,
    pub service: String,
    pub version: String,
    pub endpoints: Vec<BffEndpoint>,
}

#[derive(Debug, Deserialize)]
pub struct BffEndpoint {
    pub path: String,
    pub method: String,
    pub description: String,
}

/// Translation response
#[derive(Debug, Deserialize)]
pub struct TranslationResponse {
    pub success: bool,
    pub data: Option<TranslationData>,
    #[serde(rename = "userContext")]
    pub user_context: Option<UserContext>,
    pub delta: Option<DeltaData>,
    pub stats: Option<DeltaStats>,
    pub meta: Option<ResponseMeta>,
}

#[derive(Debug, Deserialize)]
pub struct TranslationData {
    pub messages: serde_json::Value,
    pub locale: Option<String>,
    pub version: Option<String>,
    #[serde(alias = "updated_at", alias = "updatedAt")]
    pub updated_at: Option<String>,
    pub fallback: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UserContext {
    #[serde(rename = "preferredLocale")]
    pub preferred_locale: Option<String>,
    #[serde(rename = "featureFlags")]
    pub feature_flags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct DeltaData {
    pub added: HashMap<String, String>,
    pub updated: HashMap<String, DeltaChange>,
    pub deleted: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeltaChange {
    pub old: Option<String>,
    pub new: String,
}

#[derive(Debug, Deserialize)]
pub struct DeltaStats {
    pub added: usize,
    pub updated: usize,
    pub deleted: usize,
}

#[derive(Debug, Deserialize)]
pub struct ResponseMeta {
    pub cached: bool,
    pub compressed: bool,
    #[serde(rename = "deltaSync")]
    pub delta_sync: bool,
    #[serde(rename = "responseTimeMs")]
    pub response_time_ms: u64,
}

/// Delta sync response
#[derive(Debug, Deserialize)]
pub struct DeltaSyncResponse {
    pub success: bool,
    #[serde(rename = "hasChanges")]
    pub has_changes: Option<bool>,
    pub locale: String,
    #[serde(rename = "currentVersion")]
    pub current_version: Option<String>,
    #[serde(rename = "sinceVersion")]
    pub since_version: Option<String>,
    pub delta: Option<DeltaData>,
    pub stats: Option<DeltaStats>,
}

/// Audit response
#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct UntranslatedKey {
    pub key: String,
    #[serde(rename = "englishValue")]
    pub english_value: Option<String>,
}

/// Translate batch response
#[derive(Debug, Deserialize)]
pub struct TranslateBatchResponse {
    pub success: bool,
    pub translated: Option<usize>,
    pub translations: Option<HashMap<String, String>>,
    #[serde(rename = "newVersion")]
    pub new_version: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
}

/// JSON output types
#[derive(Debug, Serialize)]
pub struct JsonHealthOutput {
    pub endpoints: Vec<EndpointHealth>,
    pub overall: String,
}

#[derive(Debug, Serialize)]
pub struct EndpointHealth {
    pub name: String,
    pub status: String,
    pub version: Option<String>,
    pub response_time_ms: Option<u64>,
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Serialize)]
pub struct JsonStatusOutput {
    pub service_health: String,
    pub version: String,
    pub bff_version: String,
    pub features: HashMap<String, bool>,
    pub locales: usize,
    pub english_keys: usize,
}

#[derive(Debug, Serialize)]
pub struct JsonTestOutput {
    pub locale: String,
    pub bff: TestResult,
    pub direct: TestResult,
    pub etag_caching: Option<bool>,
    pub delta_sync: Option<DeltaTestResult>,
}

#[derive(Debug, Serialize)]
pub struct TestResult {
    pub success: bool,
    pub keys: usize,
    pub version: Option<String>,
    pub response_time_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct DeltaTestResult {
    pub success: bool,
    pub has_changes: bool,
    pub added: usize,
    pub updated: usize,
    pub deleted: usize,
}

#[derive(Debug, Serialize)]
pub struct JsonAuditOutput {
    pub locales: Vec<LocaleAudit>,
    pub total_locales: usize,
    pub average_coverage: f64,
}

#[derive(Debug, Serialize)]
pub struct LocaleAudit {
    pub locale: String,
    pub total_keys: usize,
    pub translated: usize,
    pub untranslated: usize,
    pub coverage: f64,
    pub missing_keys: Option<Vec<String>>,
}

/// Generate InfoPlist.strings response
#[derive(Debug, Deserialize)]
pub struct GenerateInfoPlistStringsResponse {
    pub success: bool,
    pub version: Option<String>,
    pub locales: HashMap<String, HashMap<String, String>>,
    pub files: HashMap<String, String>,
    #[serde(rename = "lprojFolders")]
    pub lproj_folders: HashMap<String, String>,
    pub stats: Option<InfoPlistStats>,
    pub errors: Option<Vec<String>>,
}

/// InfoPlist.strings generation statistics
#[derive(Debug, Deserialize, Serialize)]
pub struct InfoPlistStats {
    #[serde(rename = "totalLocales")]
    pub total_locales: usize,
    #[serde(rename = "totalStrings")]
    pub total_strings: usize,
    #[serde(rename = "translatedCount")]
    pub translated_count: usize,
    #[serde(rename = "failedCount")]
    pub failed_count: usize,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
}

/// JSON output for generate-infoplist command
#[derive(Debug, Serialize)]
pub struct JsonGenerateInfoPlistOutput {
    pub success: bool,
    pub dry_run: bool,
    pub locales_generated: usize,
    pub strings_per_locale: usize,
    pub files_written: Option<Vec<String>>,
    pub stats: Option<InfoPlistStats>,
    pub errors: Option<Vec<String>>,
}
