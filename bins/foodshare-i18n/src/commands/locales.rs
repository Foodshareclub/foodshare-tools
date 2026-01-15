//! Locales command - list supported locales

use crate::api::ApiClient;
use crate::config::get_locale_info;
use anyhow::Result;
use owo_colors::OwoColorize;
use serde::Serialize;

/// JSON output for locales
#[derive(Debug, Serialize)]
struct JsonLocalesOutput {
    total: usize,
    default: String,
    locales: Vec<LocaleDetail>,
}

#[derive(Debug, Serialize)]
struct LocaleDetail {
    code: String,
    name: String,
    native_name: String,
    flag: String,
    rtl: bool,
}

/// Run locales command
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
    println!("  {}", "🌍 Supported Locales".blue().bold());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!();

    // Fetch from API to verify
    let api_locales = client.get_locales().await.ok();

    let locale_info = get_locale_info();

    // Print header
    println!(
        "  {:<6} {:<4} {:<15} {:<20} {}",
        "Code".dimmed(),
        "Flag".dimmed(),
        "Name".dimmed(),
        "Native".dimmed(),
        "RTL".dimmed()
    );
    println!(
        "  {}",
        "─".repeat(60).dimmed()
    );

    for info in &locale_info {
        let rtl_indicator = if info.rtl {
            "←".yellow().to_string()
        } else {
            " ".to_string()
        };

        // Check if locale is available from API
        let available = api_locales
            .as_ref()
            .map(|l| l.locales.contains(&info.code.to_string()))
            .unwrap_or(true);

        let code_display = if available {
            info.code.green().to_string()
        } else {
            info.code.dimmed().to_string()
        };

        println!(
            "  {:<6} {:<4} {:<15} {:<20} {}",
            code_display,
            info.flag,
            info.name,
            info.native_name,
            rtl_indicator
        );
    }

    println!();
    println!(
        "  Total: {} locales",
        locale_info.len().to_string().green()
    );

    if let Some(api) = api_locales {
        println!(
            "  Default: {}",
            api.default.cyan()
        );
    }

    println!();
    Ok(())
}

async fn run_json(client: &ApiClient) -> Result<()> {
    let api_locales = client.get_locales().await.ok();
    let locale_info = get_locale_info();

    let locales: Vec<LocaleDetail> = locale_info
        .iter()
        .map(|info| LocaleDetail {
            code: info.code.to_string(),
            name: info.name.to_string(),
            native_name: info.native_name.to_string(),
            flag: info.flag.to_string(),
            rtl: info.rtl,
        })
        .collect();

    let output = JsonLocalesOutput {
        total: locales.len(),
        default: api_locales
            .map(|l| l.default)
            .unwrap_or_else(|| "en".to_string()),
        locales,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
