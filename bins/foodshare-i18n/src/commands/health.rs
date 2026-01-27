//! Health command - check system health and status

use crate::api::ApiClient;
use crate::config::base_url;
use crate::types::{EndpointHealth, JsonHealthOutput};
use anyhow::Result;
use owo_colors::OwoColorize;
use std::collections::HashMap;

/// Run health check
pub async fn run(detailed: bool, format: &str) -> Result<()> {
    let client = ApiClient::new()?;

    if format == "json" {
        return run_json(&client, detailed).await;
    }

    if detailed {
        run_detailed(&client).await
    } else {
        run_summary(&client).await
    }
}

/// Quick summary view
async fn run_summary(client: &ApiClient) -> Result<()> {
    println!("{}", "Translation System Health".bold().cyan());
    println!("{}", "=".repeat(40).dimmed());
    println!();

    // Service health
    let (health, _) = client.health_check().await?;
    if health.status == "ok" {
        println!("Service:  {} v{}", "OK".green(), health.version);
    } else {
        println!("Service:  {}", "DOWN".red());
    }

    // BFF status
    match client.bff_info().await {
        Ok((bff, _)) => println!("BFF:      {} v{}", "OK".green(), bff.version),
        Err(_) => println!("BFF:      {}", "DOWN".red()),
    }

    // Locales
    let locales = client.get_locales().await?;
    println!("Locales:  {}", locales.locales.len());

    // English keys
    if let Ok((en_trans, _)) = client.fetch_direct_translations("en").await {
        if let Some(data) = en_trans.data {
            println!("Keys:     {}", count_keys(&data.messages));
        }
    }

    // Features
    if let Some(features) = &health.features {
        let delta = if features.delta_sync.unwrap_or(false) { "on" } else { "off" };
        let prefetch = if features.prefetch.unwrap_or(false) { "on" } else { "off" };
        println!("Features: delta={}, prefetch={}", delta, prefetch);
    }

    println!();
    println!("Run with {} for detailed endpoint checks", "--detailed".cyan());

    Ok(())
}

/// Detailed endpoint health checks
async fn run_detailed(client: &ApiClient) -> Result<()> {
    println!("{}", "Endpoint Health Check".bold().cyan());
    println!("{}", "=".repeat(40).dimmed());
    println!();

    let mut all_healthy = true;

    // BFF
    print!("BFF               ");
    match client.bff_info().await {
        Ok((info, elapsed)) => println!("{} v{} ({}ms)", "OK".green(), info.version, elapsed.as_millis()),
        Err(_) => { println!("{}", "FAIL".red()); all_healthy = false; }
    }

    // BFF translations
    print!("BFF/translations  ");
    match client.fetch_bff_translations("en", None).await {
        Ok((resp, elapsed)) if resp.success => println!("{} ({}ms)", "OK".green(), elapsed.as_millis()),
        _ => { println!("{}", "FAIL".red()); all_healthy = false; }
    }

    // get-translations
    print!("get-translations  ");
    match client.health_check().await {
        Ok((health, elapsed)) if health.status == "ok" => {
            println!("{} v{} ({}ms)", "OK".green(), health.version, elapsed.as_millis());
            if let Some(features) = &health.features {
                let delta = features.delta_sync.unwrap_or(false);
                let prefetch = features.prefetch.unwrap_or(false);
                println!("  delta_sync: {}", if delta { "on".green() } else { "off".red() });
                println!("  prefetch:   {}", if prefetch { "on".green() } else { "off".red() });
            }
        }
        _ => { println!("{}", "FAIL".red()); all_healthy = false; }
    }

    // translation-audit
    print!("translation-audit ");
    let audit_url = format!("{}/translation-audit", base_url());
    match client.check_endpoint(&audit_url).await {
        Ok((200, elapsed)) => println!("{} ({}ms)", "OK".green(), elapsed.as_millis()),
        Ok((status, _)) => { println!("{} HTTP {}", "FAIL".red(), status); all_healthy = false; }
        Err(_) => { println!("{}", "FAIL".red()); all_healthy = false; }
    }

    // delta-sync
    print!("delta-sync        ");
    match client.test_delta_sync("en", "20260101000000").await {
        Ok(resp) if resp.success => println!("{}", "OK".green()),
        _ => { println!("{}", "FAIL".red()); all_healthy = false; }
    }

    // locales
    print!("locales           ");
    match client.get_locales().await {
        Ok(resp) if resp.success => println!("{} ({} languages)", "OK".green(), resp.locales.len()),
        _ => { println!("{}", "FAIL".red()); all_healthy = false; }
    }

    println!();
    if all_healthy {
        println!("{}", "All endpoints healthy".green().bold());
    } else {
        println!("{}", "Some endpoints have issues".yellow().bold());
    }

    Ok(())
}

async fn run_json(client: &ApiClient, detailed: bool) -> Result<()> {
    let mut endpoints = Vec::new();
    let mut all_healthy = true;

    // BFF
    match client.bff_info().await {
        Ok((info, elapsed)) => endpoints.push(EndpointHealth {
            name: "bff".to_string(),
            status: "ok".to_string(),
            version: Some(info.version),
            response_time_ms: Some(elapsed.as_millis() as u64),
            features: None,
        }),
        Err(_) => {
            all_healthy = false;
            endpoints.push(EndpointHealth {
                name: "bff".to_string(),
                status: "error".to_string(),
                version: None,
                response_time_ms: None,
                features: None,
            });
        }
    }

    // get-translations
    match client.health_check().await {
        Ok((health, elapsed)) => {
            let mut features = HashMap::new();
            if let Some(f) = health.features {
                features.insert("deltaSync".to_string(), f.delta_sync.unwrap_or(false));
                features.insert("prefetch".to_string(), f.prefetch.unwrap_or(false));
            }
            endpoints.push(EndpointHealth {
                name: "get-translations".to_string(),
                status: health.status,
                version: Some(health.version),
                response_time_ms: Some(elapsed.as_millis() as u64),
                features: if detailed { Some(features) } else { None },
            });
        }
        Err(_) => {
            all_healthy = false;
            endpoints.push(EndpointHealth {
                name: "get-translations".to_string(),
                status: "error".to_string(),
                version: None,
                response_time_ms: None,
                features: None,
            });
        }
    }

    // Summary info
    let locales = client.get_locales().await.ok().map(|l| l.locales.len()).unwrap_or(0);
    let english_keys = client.fetch_direct_translations("en").await.ok()
        .and_then(|(t, _)| t.data.map(|d| count_keys(&d.messages))).unwrap_or(0);

    let output = serde_json::json!({
        "overall": if all_healthy { "healthy" } else { "degraded" },
        "endpoints": endpoints,
        "locales": locales,
        "english_keys": english_keys
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn count_keys(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(map) => {
            map.values().map(|v| if v.is_string() { 1 } else if v.is_object() { count_keys(v) } else { 0 }).sum()
        }
        _ => 0,
    }
}
