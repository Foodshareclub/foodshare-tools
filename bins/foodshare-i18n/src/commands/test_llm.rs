//! Test LLM translation endpoint
//!
//! Replaces: test-llm-endpoint.sh

use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// LLM translation request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TranslateRequest {
    text: String,
    target_language: String,
    source_language: String,
    context: String,
}

/// LLM translation response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TranslateResponse {
    translated_text: Option<String>,
    source_language: Option<String>,
    target_language: Option<String>,
    confidence: Option<f64>,
    error: Option<String>,
}

/// Run the test-llm command
pub async fn run(
    text: &str,
    source_lang: &str,
    target_lang: &str,
    context: &str,
    format: &str,
) -> Result<()> {
    let endpoint = std::env::var("LLM_TRANSLATION_ENDPOINT")
        .unwrap_or_else(|_| "https://translate.foodshare.club/api/translate".to_string());

    let api_key = std::env::var("LLM_TRANSLATION_API_KEY").ok();

    if format == "json" {
        return run_json(&endpoint, api_key.as_deref(), text, source_lang, target_lang, context).await;
    }

    println!("{}", "Testing LLM Translation Endpoint".bold().cyan());
    println!("{}", "=".repeat(40).dimmed());
    println!();

    println!("Endpoint: {}", endpoint.cyan());
    println!(
        "API Key:  {}",
        if api_key.is_some() {
            "configured".green().to_string()
        } else {
            "not set (using default)".yellow().to_string()
        }
    );
    println!();

    println!("{}", "Request:".bold());
    println!("  Text:   \"{}\"", text.cyan());
    println!("  From:   {}", source_lang);
    println!("  To:     {}", target_lang);
    println!("  Context: {}", context);
    println!();

    let start = Instant::now();
    let result = send_translation_request(
        &endpoint,
        api_key.as_deref(),
        text,
        source_lang,
        target_lang,
        context,
    )
    .await;
    let elapsed = start.elapsed();

    match result {
        Ok(response) => {
            println!("{}", "Response:".bold().green());

            if let Some(translated) = &response.translated_text {
                println!("  Translation: \"{}\"", translated.green());
            }

            if let Some(confidence) = response.confidence {
                let confidence_str = format!("{:.1}%", confidence * 100.0);
                if confidence > 0.9 {
                    println!("  Confidence:  {}", confidence_str.green());
                } else if confidence > 0.7 {
                    println!("  Confidence:  {}", confidence_str.yellow());
                } else {
                    println!("  Confidence:  {}", confidence_str.red());
                }
            }

            println!("  Response time: {}ms", elapsed.as_millis());
            println!();

            if response.error.is_some() {
                println!("{} {}", "Error:".red().bold(), response.error.unwrap());
            } else {
                println!("{}", "Translation service is working!".green().bold());
            }
        }
        Err(e) => {
            println!("{}", "Error:".red().bold());
            println!("  {}", e);
            println!();
            println!("{}", "Troubleshooting:".yellow());
            println!("  1. Check that LLM_TRANSLATION_ENDPOINT is correct");
            println!("  2. Verify LLM_TRANSLATION_API_KEY is valid");
            println!("  3. Ensure the translation service is running");
        }
    }

    println!();
    println!("{}", "Supported languages (21):".dimmed());
    println!(
        "  {}",
        "en, es, fr, de, pt, cs, ru, uk, it, pl, nl, sv,".dimmed()
    );
    println!("  {}", "zh, hi, ja, ko, vi, id, th, ar, tr".dimmed());

    Ok(())
}

/// Run in JSON output mode
async fn run_json(
    endpoint: &str,
    api_key: Option<&str>,
    text: &str,
    source_lang: &str,
    target_lang: &str,
    context: &str,
) -> Result<()> {
    let start = Instant::now();
    let result = send_translation_request(endpoint, api_key, text, source_lang, target_lang, context).await;
    let elapsed = start.elapsed();

    let output = match result {
        Ok(response) => {
            serde_json::json!({
                "success": response.error.is_none(),
                "endpoint": endpoint,
                "request": {
                    "text": text,
                    "source_language": source_lang,
                    "target_language": target_lang,
                    "context": context
                },
                "response": {
                    "translated_text": response.translated_text,
                    "confidence": response.confidence,
                    "error": response.error
                },
                "response_time_ms": elapsed.as_millis()
            })
        }
        Err(e) => {
            serde_json::json!({
                "success": false,
                "endpoint": endpoint,
                "error": e.to_string(),
                "response_time_ms": elapsed.as_millis()
            })
        }
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Send translation request to the LLM endpoint
async fn send_translation_request(
    endpoint: &str,
    api_key: Option<&str>,
    text: &str,
    source_lang: &str,
    target_lang: &str,
    context: &str,
) -> Result<TranslateResponse> {
    let client = reqwest::Client::new();

    let request = TranslateRequest {
        text: text.to_string(),
        source_language: source_lang.to_string(),
        target_language: target_lang.to_string(),
        context: context.to_string(),
    };

    let mut req_builder = client
        .post(endpoint)
        .header("Content-Type", "application/json")
        .json(&request);

    if let Some(key) = api_key {
        req_builder = req_builder.header("X-API-Key", key);
    }

    let response = req_builder
        .send()
        .await
        .context("Failed to connect to translation endpoint")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("HTTP {}: {}", status, body);
    }

    response
        .json::<TranslateResponse>()
        .await
        .context("Failed to parse response")
}
