//! Status command - show overall translation system status

use crate::api::ApiClient;
use crate::types::JsonStatusOutput;
use anyhow::Result;
use owo_colors::OwoColorize;
use std::collections::HashMap;

/// Run status check
pub async fn run(format: &str) -> Result<()> {
    let client = ApiClient::new()?;

    if format == "json" {
        return run_json(&client).await;
    }

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!("  {}", "📊 Translation System Status".blue().bold());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!();

    // Check service health
    println!("{}", "Checking service health...".yellow());
    let (health, _) = client.health_check().await?;

    if health.status == "ok" {
        println!(
            "  Service:     {} (v{})",
            "✓ Healthy".green(),
            health.version
        );
    } else {
        println!("  Service:     {}", "✗ Unhealthy".red());
    }

    // Show features
    if let Some(features) = &health.features {
        let delta = features.delta_sync.unwrap_or(false);
        let prefetch = features.prefetch.unwrap_or(false);
        println!(
            "  Delta Sync:  {}",
            if delta {
                "✓".green().to_string()
            } else {
                "✗".red().to_string()
            }
        );
        println!(
            "  Prefetch:    {}",
            if prefetch {
                "✓".green().to_string()
            } else {
                "✗".red().to_string()
            }
        );
    }

    // BFF status
    if let Ok((bff, _)) = client.bff_info().await {
        println!("  BFF:         {} (v{})", "✓".green(), bff.version);
    } else {
        println!("  BFF:         {}", "✗ Error".red());
    }

    // Fetch locale summary
    println!();
    println!("{}", "Fetching locale summary...".yellow());

    let locales = client.get_locales().await?;
    println!("  Locales:     {}", locales.locales.len());

    // Fetch English key count
    if let Ok((en_trans, _)) = client.fetch_direct_translations("en").await {
        if let Some(data) = en_trans.data {
            let key_count = count_keys(&data.messages);
            println!("  English Keys: {}", key_count.to_string().green());

            if let Some(version) = data.version {
                println!("  Version:     {}", version);
            }
        }
    }

    println!();
    Ok(())
}

async fn run_json(client: &ApiClient) -> Result<()> {
    let (health, _) = client.health_check().await?;
    let (bff, _) = client.bff_info().await?;
    let locales = client.get_locales().await?;

    let mut features = HashMap::new();
    if let Some(f) = health.features {
        features.insert("deltaSync".to_string(), f.delta_sync.unwrap_or(false));
        features.insert("prefetch".to_string(), f.prefetch.unwrap_or(false));
    }

    let mut english_keys = 0;
    if let Ok((en_trans, _)) = client.fetch_direct_translations("en").await {
        if let Some(data) = en_trans.data {
            english_keys = count_keys(&data.messages);
        }
    }

    let output = JsonStatusOutput {
        service_health: health.status,
        version: health.version,
        bff_version: bff.version,
        features,
        locales: locales.locales.len(),
        english_keys,
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
