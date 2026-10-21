//! Type definitions for API responses
//!
//! The responses the API returns are described once, in the API client.
//! This module re-exports the ones the translation commands use and
//! declares the ones the commands shape themselves.

#![allow(dead_code)]
// The re-exports below are the module's surface: the commands use some of
// them and the rest are there for the ones that will.
#![allow(unused_imports)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// The API responses. A second copy of a type is a second thing to keep
// in step with the first: the API client already describes these, field
// for field, and the commands used to convert between the two copies
// field by field.
pub use foodshare_api_client::endpoints::bff::{BffEndpoint, BffInfoResponse};
pub use foodshare_api_client::endpoints::health::{HealthFeatures, HealthResponse};
pub use foodshare_api_client::endpoints::translations::{
    AuditResponse, DeltaChange, DeltaData, DeltaStats, DeltaSyncResponse, LocalesResponse,
    ResponseMeta, TranslateBatchResponse, TranslationData, TranslationResponse, UntranslatedKey,
    UserContext,
};

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
    #[serde(default)]
    pub errors: Vec<String>,
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
    #[serde(rename = "fromCache")]
    pub from_cache: usize,
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
