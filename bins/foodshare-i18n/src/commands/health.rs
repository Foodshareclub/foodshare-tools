//! Health check command

use crate::api::ApiClient;
use crate::config::BASE_URL;
use crate::types::{EndpointHealth, JsonHealthOutput};
use anyhow::Result;
use owo_colors::OwoColorize;
use std::collections::HashMap;

/// Run health check for all endpoints
pub async fn run(timing: bool, format: &str) -> Result<()> {
    let client = ApiClient::new()?;

    if format == "json" {
        return run_json(&client, timing).await;
    }

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!("  {}", "🏥 Endpoint Health Check".blue().bold());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!();

    let mut all_healthy = true;

    // Check BFF endpoint
    print!("  BFF:               ");
    match client.bff_info().await {
        Ok((info, elapsed)) => {
            let time_str = if timing {
                format!(" ({}ms)", elapsed.as_millis())
            } else {
                String::new()
            };
            println!(
                "{} (v{}){}",
                "✓ OK".green(),
                info.version,
                time_str.dimmed()
            );
        }
        Err(_) => {
            println!("{}", "✗ Error".red());
            all_healthy = false;
        }
    }

    // Check BFF translations
    print!("  BFF/translations:  ");
    match client.fetch_bff_translations("en", None).await {
        Ok((resp, elapsed)) => {
            if resp.success {
                let time_str = if timing {
                    format!(" ({}ms)", elapsed.as_millis())
                } else {
                    String::new()
                };
                println!("{}{}", "✓ OK".green(), time_str.dimmed());
            } else {
                println!("{}", "✗ Error".red());
                all_healthy = false;
            }
        }
        Err(_) => {
            println!("{}", "✗ Error".red());
            all_healthy = false;
        }
    }

    // Check get-translations health
    print!("  get-translations:  ");
    match client.health_check().await {
        Ok((health, elapsed)) => {
            let time_str = if timing {
                format!(" ({}ms)", elapsed.as_millis())
            } else {
                String::new()
            };
            if health.status == "ok" {
                println!(
                    "{} (v{}){}",
                    "✓ OK".green(),
                    health.version,
                    time_str.dimmed()
                );

                // Show features
                if let Some(features) = &health.features {
                    let delta = features.delta_sync.unwrap_or(false);
                    let prefetch = features.prefetch.unwrap_or(false);
                    println!(
                        "    └─ Delta Sync:   {}",
                        if delta {
                            "✓".green().to_string()
                        } else {
                            "✗".red().to_string()
                        }
                    );
                    println!(
                        "    └─ Prefetch:     {}",
                        if prefetch {
                            "✓".green().to_string()
                        } else {
                            "✗".red().to_string()
                        }
                    );
                }
            } else {
                println!("{}", "✗ Unhealthy".red());
                all_healthy = false;
            }
        }
        Err(_) => {
            println!("{}", "✗ Error".red());
            all_healthy = false;
        }
    }

    // Check translation-audit
    print!("  translation-audit: ");
    let audit_url = format!("{}/translation-audit", BASE_URL);
    match client.check_endpoint(&audit_url).await {
        Ok((status, elapsed)) => {
            let time_str = if timing {
                format!(" ({}ms)", elapsed.as_millis())
            } else {
                String::new()
            };
            if status == 200 {
                println!("{}{}", "✓ OK".green(), time_str.dimmed());
            } else {
                println!("{} (HTTP {})", "✗ Error".red(), status);
                all_healthy = false;
            }
        }
        Err(_) => {
            println!("{}", "✗ Error".red());
            all_healthy = false;
        }
    }

    // Check delta sync endpoint
    print!("  delta-sync:        ");
    match client.test_delta_sync("en", "20260101000000").await {
        Ok(resp) => {
            if resp.success {
                println!("{}", "✓ OK".green());
            } else {
                println!("{}", "✗ Error".red());
                all_healthy = false;
            }
        }
        Err(_) => {
            println!("{}", "✗ Error".red());
            all_healthy = false;
        }
    }

    // Check locales endpoint
    print!("  locales:           ");
    match client.get_locales().await {
        Ok(resp) => {
            if resp.success {
                println!("{} ({} languages)", "✓ OK".green(), resp.locales.len());
            } else {
                println!("{}", "✗ Error".red());
                all_healthy = false;
            }
        }
        Err(_) => {
            println!("{}", "✗ Error".red());
            all_healthy = false;
        }
    }

    println!();
    if all_healthy {
        println!("  {} All endpoints healthy", "✓".green().bold());
    } else {
        println!("  {} Some endpoints have issues", "⚠".yellow().bold());
    }
    println!();

    Ok(())
}

async fn run_json(client: &ApiClient, _timing: bool) -> Result<()> {
    let mut endpoints = Vec::new();
    let mut all_healthy = true;

    // BFF
    if let Ok((info, elapsed)) = client.bff_info().await {
        endpoints.push(EndpointHealth {
            name: "bff".to_string(),
            status: "ok".to_string(),
            version: Some(info.version),
            response_time_ms: Some(elapsed.as_millis() as u64),
            features: None,
        });
    } else {
        all_healthy = false;
        endpoints.push(EndpointHealth {
            name: "bff".to_string(),
            status: "error".to_string(),
            version: None,
            response_time_ms: None,
            features: None,
        });
    }

    // get-translations
    if let Ok((health, elapsed)) = client.health_check().await {
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
            features: Some(features),
        });
    } else {
        all_healthy = false;
        endpoints.push(EndpointHealth {
            name: "get-translations".to_string(),
            status: "error".to_string(),
            version: None,
            response_time_ms: None,
            features: None,
        });
    }

    let output = JsonHealthOutput {
        endpoints,
        overall: if all_healthy {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        },
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
