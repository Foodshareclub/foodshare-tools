//! Translate command - auto-translate missing keys

use crate::api::ApiClient;
use crate::config::SUPPORTED_LOCALES;
use anyhow::Result;
use owo_colors::OwoColorize;
use serde::Serialize;

/// JSON output for translate command
#[derive(Debug, Serialize)]
struct JsonTranslateOutput {
    locale: String,
    dry_run: bool,
    translated: usize,
    translations: Option<std::collections::HashMap<String, String>>,
    new_version: Option<String>,
    error: Option<String>,
}

/// JSON output for sync all command
#[derive(Debug, Serialize)]
struct JsonSyncOutput {
    dry_run: bool,
    locales_processed: usize,
    total_translated: usize,
    results: Vec<LocaleSyncResult>,
}

#[derive(Debug, Serialize)]
struct LocaleSyncResult {
    locale: String,
    translated: usize,
    success: bool,
    error: Option<String>,
}

/// Run translate command for a single locale
pub async fn run(locale: &str, apply: bool, limit: usize, format: &str) -> Result<()> {
    let client = ApiClient::new()?;

    if format == "json" {
        return run_json(&client, locale, apply, limit).await;
    }

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!(
        "  {}",
        format!("🌐 Auto-Translate: {}", locale).blue().bold()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!();

    if !apply {
        println!(
            "  {} Running in dry-run mode (use --apply to save)",
            "ℹ".cyan()
        );
        println!();
    }

    // First, audit to find missing keys
    println!("{}", "Finding missing translations...".yellow());
    let audit = client.audit_locale(locale, limit).await?;

    let untranslated_count = audit.untranslated_count.unwrap_or(0);
    if untranslated_count == 0 {
        println!(
            "  {} No missing translations for {}",
            "✓".green(),
            locale
        );
        println!();
        return Ok(());
    }

    println!(
        "  Found {} missing keys (processing up to {})",
        untranslated_count.to_string().yellow(),
        limit
    );
    println!();

    // Get the missing keys
    let missing_keys = audit.untranslated.unwrap_or_default();
    if missing_keys.is_empty() {
        println!("  {} No keys to translate", "ℹ".cyan());
        return Ok(());
    }

    // Build keys object for translation
    let keys: serde_json::Value = missing_keys
        .iter()
        .take(limit)
        .filter_map(|k| {
            k.english_value
                .as_ref()
                .map(|v| (k.key.clone(), serde_json::Value::String(v.clone())))
        })
        .collect::<serde_json::Map<String, serde_json::Value>>()
        .into();

    println!("{}", "Translating...".yellow());

    // Call translate batch
    match client.translate_batch(locale, &keys, apply).await {
        Ok(resp) => {
            if resp.success {
                let translated = resp.translated.unwrap_or(0);
                if apply {
                    println!(
                        "  {} Translated and saved {} keys",
                        "✓".green(),
                        translated.to_string().green()
                    );
                    if let Some(version) = resp.new_version {
                        println!("    └─ New version: {}", version);
                    }
                } else {
                    println!(
                        "  {} Would translate {} keys (dry-run)",
                        "✓".green(),
                        translated.to_string().cyan()
                    );

                    // Show sample translations
                    if let Some(translations) = resp.translations {
                        println!();
                        println!("  Sample translations:");
                        for (key, value) in translations.iter().take(5) {
                            println!("    {} → {}", key.dimmed(), value.cyan());
                        }
                        if translations.len() > 5 {
                            println!("    ... and {} more", translations.len() - 5);
                        }
                    }
                }
            } else {
                let error = resp.error.unwrap_or_else(|| "Unknown error".to_string());
                println!("  {} Translation failed: {}", "✗".red(), error);
            }
        }
        Err(e) => {
            println!("  {} Translation error: {}", "✗".red(), e);
        }
    }

    println!();
    Ok(())
}

async fn run_json(client: &ApiClient, locale: &str, apply: bool, limit: usize) -> Result<()> {
    // Audit to find missing keys
    let audit = client.audit_locale(locale, limit).await?;
    let missing_keys = audit.untranslated.unwrap_or_default();

    if missing_keys.is_empty() {
        let output = JsonTranslateOutput {
            locale: locale.to_string(),
            dry_run: !apply,
            translated: 0,
            translations: None,
            new_version: None,
            error: None,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Build keys object
    let keys: serde_json::Value = missing_keys
        .iter()
        .take(limit)
        .filter_map(|k| {
            k.english_value
                .as_ref()
                .map(|v| (k.key.clone(), serde_json::Value::String(v.clone())))
        })
        .collect::<serde_json::Map<String, serde_json::Value>>()
        .into();

    // Translate
    match client.translate_batch(locale, &keys, apply).await {
        Ok(resp) => {
            let output = JsonTranslateOutput {
                locale: locale.to_string(),
                dry_run: !apply,
                translated: resp.translated.unwrap_or(0),
                translations: resp.translations,
                new_version: resp.new_version,
                error: resp.error,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        Err(e) => {
            let output = JsonTranslateOutput {
                locale: locale.to_string(),
                dry_run: !apply,
                translated: 0,
                translations: None,
                new_version: None,
                error: Some(e.to_string()),
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}

/// Sync all locales
pub async fn sync_all(apply: bool, format: &str) -> Result<()> {
    let client = ApiClient::new()?;

    if format == "json" {
        return sync_all_json(&client, apply).await;
    }

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!("  {}", "🔄 Sync All Locales".blue().bold());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!();

    if !apply {
        println!(
            "  {} Running in dry-run mode (use --apply to save)",
            "ℹ".cyan()
        );
        println!();
    }

    let mut total_translated = 0;
    let mut success_count = 0;
    let mut error_count = 0;

    for locale in SUPPORTED_LOCALES {
        print!("  {:<5}: ", locale);

        // Audit locale
        match client.audit_locale(locale, 50).await {
            Ok(audit) => {
                let missing_keys = audit.untranslated.unwrap_or_default();
                if missing_keys.is_empty() {
                    println!("{}", "✓ Complete".green());
                    success_count += 1;
                    continue;
                }

                // Build keys object
                let keys: serde_json::Value = missing_keys
                    .iter()
                    .take(50)
                    .filter_map(|k| {
                        k.english_value
                            .as_ref()
                            .map(|v| (k.key.clone(), serde_json::Value::String(v.clone())))
                    })
                    .collect::<serde_json::Map<String, serde_json::Value>>()
                    .into();

                // Translate
                match client.translate_batch(locale, &keys, apply).await {
                    Ok(resp) => {
                        if resp.success {
                            let count = resp.translated.unwrap_or(0);
                            total_translated += count;
                            if apply {
                                println!(
                                    "{} ({} keys translated)",
                                    "✓".green(),
                                    count.to_string().green()
                                );
                            } else {
                                println!(
                                    "{} ({} keys would be translated)",
                                    "○".cyan(),
                                    count.to_string().cyan()
                                );
                            }
                            success_count += 1;
                        } else {
                            let error = resp.error.unwrap_or_else(|| "Unknown".to_string());
                            println!("{} ({})", "✗".red(), error.dimmed());
                            error_count += 1;
                        }
                    }
                    Err(e) => {
                        println!("{} ({})", "✗".red(), e.to_string().dimmed());
                        error_count += 1;
                    }
                }
            }
            Err(e) => {
                println!("{} ({})", "✗".red(), e.to_string().dimmed());
                error_count += 1;
            }
        }
    }

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!(
        "  Summary: {} locales processed, {} total translations{}",
        success_count.to_string().green(),
        total_translated.to_string().cyan(),
        if error_count > 0 {
            format!(", {} errors", error_count.to_string().red())
        } else {
            String::new()
        }
    );
    println!();

    Ok(())
}

async fn sync_all_json(client: &ApiClient, apply: bool) -> Result<()> {
    let mut results = Vec::new();
    let mut total_translated = 0;

    for locale in SUPPORTED_LOCALES {
        match client.audit_locale(locale, 50).await {
            Ok(audit) => {
                let missing_keys = audit.untranslated.unwrap_or_default();
                if missing_keys.is_empty() {
                    results.push(LocaleSyncResult {
                        locale: locale.to_string(),
                        translated: 0,
                        success: true,
                        error: None,
                    });
                    continue;
                }

                let keys: serde_json::Value = missing_keys
                    .iter()
                    .take(50)
                    .filter_map(|k| {
                        k.english_value
                            .as_ref()
                            .map(|v| (k.key.clone(), serde_json::Value::String(v.clone())))
                    })
                    .collect::<serde_json::Map<String, serde_json::Value>>()
                    .into();

                match client.translate_batch(locale, &keys, apply).await {
                    Ok(resp) => {
                        let count = resp.translated.unwrap_or(0);
                        total_translated += count;
                        results.push(LocaleSyncResult {
                            locale: locale.to_string(),
                            translated: count,
                            success: resp.success,
                            error: resp.error,
                        });
                    }
                    Err(e) => {
                        results.push(LocaleSyncResult {
                            locale: locale.to_string(),
                            translated: 0,
                            success: false,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
            Err(e) => {
                results.push(LocaleSyncResult {
                    locale: locale.to_string(),
                    translated: 0,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    let output = JsonSyncOutput {
        dry_run: !apply,
        locales_processed: results.len(),
        total_translated,
        results,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
