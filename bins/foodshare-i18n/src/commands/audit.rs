//! Audit command - check translation coverage

use crate::api::ApiClient;
use crate::config::SUPPORTED_LOCALES;
use crate::types::{JsonAuditOutput, LocaleAudit};
use anyhow::Result;
use owo_colors::OwoColorize;

/// Run translation audit
pub async fn run(locale: Option<&str>, show_missing: bool, limit: usize, format: &str) -> Result<()> {
    let client = ApiClient::new()?;

    if format == "json" {
        return run_json(&client, locale, show_missing, limit).await;
    }

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!("  {}", "🔍 Translation Coverage Audit".blue().bold());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!();

    // Get English key count as reference
    let (en_trans, _) = client.fetch_direct_translations("en").await?;
    let en_keys = en_trans
        .data
        .as_ref()
        .map(|d| count_keys(&d.messages))
        .unwrap_or(0);
    println!(
        "Reference: English has {} translation keys",
        en_keys.to_string().green()
    );
    println!();

    if let Some(loc) = locale {
        // Audit single locale
        audit_single_locale(&client, loc, en_keys, show_missing, limit).await?;
    } else {
        // Audit all locales
        for loc in SUPPORTED_LOCALES {
            audit_single_locale(&client, loc, en_keys, show_missing, limit).await?;
        }
    }

    println!();
    Ok(())
}

async fn audit_single_locale(
    client: &ApiClient,
    locale: &str,
    en_keys: usize,
    show_missing: bool,
    limit: usize,
) -> Result<()> {
    // Fetch translations for this locale
    match client.fetch_direct_translations(locale).await {
        Ok((resp, _)) => {
            if let Some(data) = resp.data {
                let key_count = count_keys(&data.messages);
                let coverage = if en_keys > 0 {
                    (key_count as f64 / en_keys as f64) * 100.0
                } else {
                    0.0
                };

                let color = if coverage >= 90.0 {
                    "green"
                } else if coverage >= 70.0 {
                    "yellow"
                } else {
                    "red"
                };

                let coverage_str = format!("{:5.1}%", coverage);
                let coverage_colored = match color {
                    "green" => coverage_str.green().to_string(),
                    "yellow" => coverage_str.yellow().to_string(),
                    _ => coverage_str.red().to_string(),
                };

                println!(
                    "  {:<5}: {} coverage ({}/{} keys)",
                    locale, coverage_colored, key_count, en_keys
                );

                // Show missing keys if requested
                if show_missing && key_count < en_keys {
                    // Try to get audit info
                    if let Ok(audit) = client.audit_locale(locale, limit).await {
                        if let Some(untranslated) = audit.untranslated {
                            let missing: Vec<_> = untranslated
                                .iter()
                                .take(limit)
                                .map(|k| k.key.as_str())
                                .collect();
                            if !missing.is_empty() {
                                println!(
                                    "         Missing: {}",
                                    missing.join(", ").dimmed()
                                );
                            }
                        }
                    }
                }
            }
        }
        Err(_) => {
            println!("  {:<5}: {}", locale, "Error fetching".red());
        }
    }

    Ok(())
}

async fn run_json(
    client: &ApiClient,
    locale: Option<&str>,
    show_missing: bool,
    limit: usize,
) -> Result<()> {
    // Get English key count
    let (en_trans, _) = client.fetch_direct_translations("en").await?;
    let en_keys = en_trans
        .data
        .as_ref()
        .map(|d| count_keys(&d.messages))
        .unwrap_or(0);

    let locales_to_audit: Vec<&str> = if let Some(loc) = locale {
        vec![loc]
    } else {
        SUPPORTED_LOCALES.to_vec()
    };

    let mut audits = Vec::new();
    let mut total_coverage = 0.0;

    for loc in &locales_to_audit {
        if let Ok((resp, _)) = client.fetch_direct_translations(loc).await {
            if let Some(data) = resp.data {
                let key_count = count_keys(&data.messages);
                let coverage = if en_keys > 0 {
                    (key_count as f64 / en_keys as f64) * 100.0
                } else {
                    0.0
                };
                total_coverage += coverage;

                let missing_keys = if show_missing && key_count < en_keys {
                    client
                        .audit_locale(loc, limit)
                        .await
                        .ok()
                        .and_then(|a| a.untranslated)
                        .map(|u| u.iter().take(limit).map(|k| k.key.clone()).collect())
                } else {
                    None
                };

                audits.push(LocaleAudit {
                    locale: loc.to_string(),
                    total_keys: en_keys,
                    translated: key_count,
                    untranslated: en_keys.saturating_sub(key_count),
                    coverage,
                    missing_keys,
                });
            }
        }
    }

    let avg_coverage = if !audits.is_empty() {
        total_coverage / audits.len() as f64
    } else {
        0.0
    };

    let output = JsonAuditOutput {
        locales: audits,
        total_locales: locales_to_audit.len(),
        average_coverage: avg_coverage,
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
