//! Test command - test translation fetch for a locale

use crate::api::ApiClient;
use crate::types::{DeltaTestResult, JsonTestOutput, TestResult};
use anyhow::Result;
use owo_colors::OwoColorize;

/// Run translation fetch test
pub async fn run(locale: &str, delta: bool, cache: bool, format: &str) -> Result<()> {
    let client = ApiClient::new()?;

    if format == "json" {
        return run_json(&client, locale, delta, cache).await;
    }

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!(
        "  {}",
        format!("🧪 Testing Translation Fetch: {}", locale)
            .blue()
            .bold()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!();

    // Test BFF endpoint
    println!("{}", "Testing BFF endpoint...".yellow());
    match client.fetch_bff_translations(locale, None).await {
        Ok((resp, elapsed)) => {
            if resp.success {
                if let Some(data) = &resp.data {
                    let key_count = count_keys(&data.messages);
                    let version = data.version.as_deref().unwrap_or("unknown");
                    println!(
                        "  {} BFF: {} keys in {}ms (v{})",
                        "✓".green(),
                        key_count.to_string().cyan(),
                        elapsed.as_millis(),
                        version
                    );

                    // Show meta info if available
                    if let Some(meta) = &resp.meta {
                        println!(
                            "    └─ Cached: {}, Delta: {}",
                            meta.cached,
                            meta.delta_sync
                        );
                    }
                }
            } else {
                println!("  {} BFF failed", "✗".red());
            }
        }
        Err(e) => {
            println!("  {} BFF error: {}", "✗".red(), e);
        }
    }

    // Test direct endpoint
    println!();
    println!("{}", "Testing direct endpoint...".yellow());
    let direct_version;
    match client.fetch_direct_translations(locale).await {
        Ok((resp, elapsed)) => {
            if resp.success {
                if let Some(data) = &resp.data {
                    let key_count = count_keys(&data.messages);
                    direct_version = data.version.clone();
                    let version = data.version.as_deref().unwrap_or("unknown");
                    println!(
                        "  {} Direct: {} keys in {}ms (v{})",
                        "✓".green(),
                        key_count.to_string().cyan(),
                        elapsed.as_millis(),
                        version
                    );
                } else {
                    direct_version = None;
                }
            } else {
                println!("  {} Direct failed", "✗".red());
                direct_version = None;
            }
        }
        Err(e) => {
            println!("  {} Direct error: {}", "✗".red(), e);
            direct_version = None;
        }
    }

    // Test ETag caching
    if cache {
        println!();
        println!("{}", "Testing ETag caching...".yellow());
        if let Some(version) = &direct_version {
            match client.test_etag_caching(locale, version).await {
                Ok(status) => {
                    if status == 304 {
                        println!(
                            "  {} ETag caching working (304 Not Modified)",
                            "✓".green()
                        );
                    } else {
                        println!(
                            "  {} ETag returned HTTP {} (expected 304)",
                            "?".yellow(),
                            status
                        );
                    }
                }
                Err(e) => {
                    println!("  {} ETag test error: {}", "✗".red(), e);
                }
            }
        } else {
            println!("  {} No version available for ETag test", "⚠".yellow());
        }
    }

    // Test delta sync
    if delta {
        println!();
        println!("{}", "Testing delta sync...".yellow());
        match client.test_delta_sync(locale, "20260101000000").await {
            Ok(resp) => {
                if resp.success {
                    let has_changes = resp.has_changes.unwrap_or(false);
                    if has_changes {
                        if let Some(stats) = &resp.stats {
                            println!(
                                "  {} Delta sync: +{} added, ~{} updated, -{} deleted",
                                "✓".green(),
                                stats.added.to_string().green(),
                                stats.updated.to_string().yellow(),
                                stats.deleted.to_string().red()
                            );
                        }
                    } else {
                        println!("  {} Delta sync: No changes", "✓".green());
                    }
                    if let Some(version) = &resp.current_version {
                        println!("    └─ Current version: {}", version);
                    }
                } else {
                    println!("  {} Delta sync failed", "✗".red());
                }
            }
            Err(e) => {
                println!("  {} Delta sync error: {}", "✗".red(), e);
            }
        }
    }

    println!();
    Ok(())
}

async fn run_json(client: &ApiClient, locale: &str, delta: bool, cache: bool) -> Result<()> {
    // BFF test
    let bff_result = match client.fetch_bff_translations(locale, None).await {
        Ok((resp, elapsed)) => {
            let keys = resp
                .data
                .as_ref()
                .map(|d| count_keys(&d.messages))
                .unwrap_or(0);
            let version = resp.data.as_ref().and_then(|d| d.version.clone());
            TestResult {
                success: resp.success,
                keys,
                version,
                response_time_ms: elapsed.as_millis() as u64,
            }
        }
        Err(_) => TestResult {
            success: false,
            keys: 0,
            version: None,
            response_time_ms: 0,
        },
    };

    // Direct test
    let (direct_result, direct_version) = match client.fetch_direct_translations(locale).await {
        Ok((resp, elapsed)) => {
            let keys = resp
                .data
                .as_ref()
                .map(|d| count_keys(&d.messages))
                .unwrap_or(0);
            let version = resp.data.as_ref().and_then(|d| d.version.clone());
            (
                TestResult {
                    success: resp.success,
                    keys,
                    version: version.clone(),
                    response_time_ms: elapsed.as_millis() as u64,
                },
                version,
            )
        }
        Err(_) => (
            TestResult {
                success: false,
                keys: 0,
                version: None,
                response_time_ms: 0,
            },
            None,
        ),
    };

    // ETag test
    let etag_caching = if cache {
        if let Some(version) = &direct_version {
            client
                .test_etag_caching(locale, version)
                .await
                .ok()
                .map(|status| status == 304)
        } else {
            None
        }
    } else {
        None
    };

    // Delta test
    let delta_sync = if delta {
        client
            .test_delta_sync(locale, "20260101000000")
            .await
            .ok()
            .map(|resp| DeltaTestResult {
                success: resp.success,
                has_changes: resp.has_changes.unwrap_or(false),
                added: resp.stats.as_ref().map(|s| s.added).unwrap_or(0),
                updated: resp.stats.as_ref().map(|s| s.updated).unwrap_or(0),
                deleted: resp.stats.as_ref().map(|s| s.deleted).unwrap_or(0),
            })
    } else {
        None
    };

    let output = JsonTestOutput {
        locale: locale.to_string(),
        bff: bff_result,
        direct: direct_result,
        etag_caching,
        delta_sync,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Count keys in a nested JSON object
fn count_keys(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(map) => {
            let mut count = 0;
            for v in map.values() {
                if v.is_string() {
                    count += 1;
                } else if v.is_object() {
                    count += count_keys(v);
                }
            }
            count
        }
        _ => 0,
    }
}
