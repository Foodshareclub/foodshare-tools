//! Test commands - test translation system components

use crate::api::ApiClient;
use crate::types::{DeltaTestResult, JsonTestOutput, TestResult};
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{Duration, Instant};

// ============================================================================
// Fetch Test (test fetch)
// ============================================================================

/// Run translation fetch test
pub async fn run(locale: &str, delta: bool, cache: bool, format: &str) -> Result<()> {
    let client = ApiClient::new()?;

    if format == "json" {
        return run_json(&client, locale, delta, cache).await;
    }

    println!("{}", "Testing Translation Fetch".bold().cyan());
    println!("{}", "=".repeat(40).dimmed());
    println!("Locale: {}", locale.cyan());
    println!();

    // Test BFF endpoint
    println!("{}", "BFF endpoint...".yellow());
    match client.fetch_bff_translations(locale, None).await {
        Ok((resp, elapsed)) => {
            if resp.success {
                if let Some(data) = &resp.data {
                    let key_count = count_keys(&data.messages);
                    let version = data.version.as_deref().unwrap_or("unknown");
                    println!(
                        "  {} {} keys in {}ms (v{})",
                        "OK".green(),
                        key_count.to_string().cyan(),
                        elapsed.as_millis(),
                        version
                    );
                }
            } else {
                println!("  {} BFF failed", "FAIL".red());
            }
        }
        Err(e) => println!("  {} {}", "FAIL".red(), e),
    }

    // Test direct endpoint
    println!("{}", "Direct endpoint...".yellow());
    let direct_version;
    match client.fetch_direct_translations(locale).await {
        Ok((resp, elapsed)) => {
            if resp.success {
                if let Some(data) = &resp.data {
                    let key_count = count_keys(&data.messages);
                    direct_version = data.version.clone();
                    let version = data.version.as_deref().unwrap_or("unknown");
                    println!(
                        "  {} {} keys in {}ms (v{})",
                        "OK".green(),
                        key_count.to_string().cyan(),
                        elapsed.as_millis(),
                        version
                    );
                } else {
                    direct_version = None;
                }
            } else {
                println!("  {} Direct failed", "FAIL".red());
                direct_version = None;
            }
        }
        Err(e) => {
            println!("  {} {}", "FAIL".red(), e);
            direct_version = None;
        }
    }

    // Test ETag caching
    if cache {
        println!("{}", "ETag caching...".yellow());
        if let Some(version) = &direct_version {
            match client.test_etag_caching(locale, version).await {
                Ok(status) => {
                    if status == 304 {
                        println!("  {} 304 Not Modified", "OK".green());
                    } else {
                        println!("  {} HTTP {} (expected 304)", "WARN".yellow(), status);
                    }
                }
                Err(e) => println!("  {} {}", "FAIL".red(), e),
            }
        } else {
            println!("  {} No version for ETag test", "SKIP".yellow());
        }
    }

    // Test delta sync
    if delta {
        println!("{}", "Delta sync...".yellow());
        match client.test_delta_sync(locale, "20260101000000").await {
            Ok(resp) => {
                if resp.success {
                    if let Some(stats) = &resp.stats {
                        println!(
                            "  {} +{} ~{} -{}",
                            "OK".green(),
                            stats.added.to_string().green(),
                            stats.updated.to_string().yellow(),
                            stats.deleted.to_string().red()
                        );
                    }
                } else {
                    println!("  {} Delta sync failed", "FAIL".red());
                }
            }
            Err(e) => println!("  {} {}", "FAIL".red(), e),
        }
    }

    Ok(())
}

async fn run_json(client: &ApiClient, locale: &str, delta: bool, cache: bool) -> Result<()> {
    let bff_result = match client.fetch_bff_translations(locale, None).await {
        Ok((resp, elapsed)) => {
            let keys = resp.data.as_ref().map(|d| count_keys(&d.messages)).unwrap_or(0);
            let version = resp.data.as_ref().and_then(|d| d.version.clone());
            TestResult { success: resp.success, keys, version, response_time_ms: elapsed.as_millis() as u64 }
        }
        Err(_) => TestResult { success: false, keys: 0, version: None, response_time_ms: 0 },
    };

    let (direct_result, direct_version) = match client.fetch_direct_translations(locale).await {
        Ok((resp, elapsed)) => {
            let keys = resp.data.as_ref().map(|d| count_keys(&d.messages)).unwrap_or(0);
            let version = resp.data.as_ref().and_then(|d| d.version.clone());
            (TestResult { success: resp.success, keys, version: version.clone(), response_time_ms: elapsed.as_millis() as u64 }, version)
        }
        Err(_) => (TestResult { success: false, keys: 0, version: None, response_time_ms: 0 }, None),
    };

    let etag_caching = if cache {
        if let Some(version) = &direct_version {
            client.test_etag_caching(locale, version).await.ok().map(|status| status == 304)
        } else { None }
    } else { None };

    let delta_sync = if delta {
        client.test_delta_sync(locale, "20260101000000").await.ok().map(|resp| DeltaTestResult {
            success: resp.success,
            has_changes: resp.has_changes.unwrap_or(false),
            added: resp.stats.as_ref().map(|s| s.added).unwrap_or(0),
            updated: resp.stats.as_ref().map(|s| s.updated).unwrap_or(0),
            deleted: resp.stats.as_ref().map(|s| s.deleted).unwrap_or(0),
        })
    } else { None };

    let output = JsonTestOutput { locale: locale.to_string(), bff: bff_result, direct: direct_result, etag_caching, delta_sync };
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

// ============================================================================
// LLM Test (test llm)
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LlmTranslateRequest {
    text: String,
    target_language: String,
    source_language: String,
    context: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LlmTranslateResponse {
    translated_text: Option<String>,
    confidence: Option<f64>,
    error: Option<String>,
}

pub async fn run_llm(text: &str, source: &str, target: &str, context: &str, format: &str) -> Result<()> {
    let endpoint = std::env::var("LLM_TRANSLATION_ENDPOINT")
        .unwrap_or_else(|_| "https://translate.foodshare.club/api/translate".to_string());
    let api_key = std::env::var("LLM_TRANSLATION_API_KEY").ok();

    if format == "json" {
        return run_llm_json(&endpoint, api_key.as_deref(), text, source, target, context).await;
    }

    println!("{}", "Testing LLM Translation".bold().cyan());
    println!("{}", "=".repeat(40).dimmed());
    println!("Endpoint: {}", endpoint.cyan());
    println!("Text: \"{}\"", text);
    println!("{} -> {}", source, target);
    println!();

    let start = Instant::now();
    let result = send_llm_request(&endpoint, api_key.as_deref(), text, source, target, context).await;
    let elapsed = start.elapsed();

    match result {
        Ok(resp) => {
            if let Some(translated) = &resp.translated_text {
                println!("{} \"{}\"", "OK".green(), translated.green());
            }
            if let Some(conf) = resp.confidence {
                println!("Confidence: {:.0}%", conf * 100.0);
            }
            println!("Time: {}ms", elapsed.as_millis());
            if let Some(err) = resp.error {
                println!("{} {}", "Error:".red(), err);
            }
        }
        Err(e) => println!("{} {}", "FAIL".red(), e),
    }

    Ok(())
}

async fn run_llm_json(endpoint: &str, api_key: Option<&str>, text: &str, source: &str, target: &str, context: &str) -> Result<()> {
    let start = Instant::now();
    let result = send_llm_request(endpoint, api_key, text, source, target, context).await;
    let elapsed = start.elapsed();

    let output = match result {
        Ok(resp) => json!({ "success": resp.error.is_none(), "translated_text": resp.translated_text, "confidence": resp.confidence, "response_time_ms": elapsed.as_millis() }),
        Err(e) => json!({ "success": false, "error": e.to_string(), "response_time_ms": elapsed.as_millis() }),
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

async fn send_llm_request(endpoint: &str, api_key: Option<&str>, text: &str, source: &str, target: &str, context: &str) -> Result<LlmTranslateResponse> {
    let client = reqwest::Client::new();
    let request = LlmTranslateRequest { text: text.to_string(), source_language: source.to_string(), target_language: target.to_string(), context: context.to_string() };

    let mut req = client.post(endpoint).header("Content-Type", "application/json").json(&request);
    if let Some(key) = api_key { req = req.header("X-API-Key", key); }

    let response = req.send().await.context("Failed to connect")?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("HTTP {}: {}", status, body);
    }
    response.json().await.context("Failed to parse response")
}

// ============================================================================
// Posts Test (test posts)
// ============================================================================

#[derive(Debug, Deserialize)]
struct Post {
    id: i64,
    post_name: String,
    post_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TranslationResponse {
    success: bool,
    #[serde(rename = "fromRedis")]
    from_redis: Option<i32>,
    #[serde(rename = "fromDatabase")]
    from_database: Option<i32>,
    #[serde(rename = "onDemand")]
    on_demand: Option<i32>,
    #[serde(rename = "notFound")]
    not_found: Option<i32>,
}

#[derive(Debug, Serialize)]
struct TranslateBatchRequest {
    content_type: String,
    content_id: String,
    fields: Vec<TranslationField>,
}

#[derive(Debug, Serialize)]
struct TranslationField {
    name: String,
    text: String,
}

pub async fn run_posts(locale: &str, limit: usize, skip_trigger: bool, verbose: bool, format: &str) -> Result<()> {
    let supabase_url = std::env::var("SUPABASE_URL").context("SUPABASE_URL required")?;
    let anon_key = std::env::var("SUPABASE_ANON_KEY").context("SUPABASE_ANON_KEY required")?;
    let service_key = std::env::var("SUPABASE_SERVICE_ROLE_KEY").ok();

    if format == "json" {
        return run_posts_json(&supabase_url, &anon_key, service_key.as_deref(), locale, limit, skip_trigger).await;
    }

    println!("{}", "Testing Post Translation System".bold().cyan());
    println!("{}", "=".repeat(40).dimmed());
    println!("Locale: {}, Limit: {}", locale.cyan(), limit);
    println!();

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // Check schema
    print!("Schema check... ");
    check_schema(&client, &supabase_url, &anon_key).await?;
    println!("{}", "OK".green());

    // Fetch posts
    print!("Fetching posts... ");
    let posts = fetch_posts(&client, &supabase_url, &anon_key, limit).await?;
    println!("{} {} posts", "OK".green(), posts.len());

    if verbose {
        for post in &posts {
            println!("  - {}: {}", post.id, post.post_name);
        }
    }

    // Trigger translations
    if !skip_trigger && service_key.is_some() {
        print!("Triggering translations... ");
        let key = service_key.as_ref().unwrap();
        for post in &posts {
            trigger_translation(&client, &supabase_url, key, post).await?;
        }
        println!("{}", "OK".green());
        println!("Waiting 30s for processing...");
        tokio::time::sleep(Duration::from_secs(30)).await;
    }

    // Fetch translations
    print!("Fetching translations... ");
    let tr = fetch_translations(&client, &supabase_url, &anon_key, &posts, locale).await?;
    println!("{}", "OK".green());
    println!(
        "  Redis: {}, DB: {}, LLM: {}, Missing: {}",
        tr.from_redis.unwrap_or(0),
        tr.from_database.unwrap_or(0),
        tr.on_demand.unwrap_or(0),
        tr.not_found.unwrap_or(0).to_string().red()
    );

    println!();
    println!("{}", "All tests passed!".green().bold());

    Ok(())
}

async fn run_posts_json(supabase_url: &str, anon_key: &str, service_key: Option<&str>, locale: &str, limit: usize, skip_trigger: bool) -> Result<()> {
    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    let schema_ok = check_schema(&client, supabase_url, anon_key).await.is_ok();
    let posts = fetch_posts(&client, supabase_url, anon_key, limit).await.unwrap_or_default();

    if !skip_trigger && service_key.is_some() {
        let key = service_key.unwrap();
        for post in &posts {
            let _ = trigger_translation(&client, supabase_url, key, post).await;
        }
        tokio::time::sleep(Duration::from_secs(30)).await;
    }

    let tr = fetch_translations(&client, supabase_url, anon_key, &posts, locale).await.ok();

    let output = json!({
        "schema_ok": schema_ok,
        "posts_found": posts.len(),
        "translations": tr.map(|t| json!({
            "from_redis": t.from_redis,
            "from_database": t.from_database,
            "on_demand": t.on_demand,
            "not_found": t.not_found
        }))
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

async fn check_schema(client: &Client, url: &str, key: &str) -> Result<()> {
    let resp = client.get(format!("{}/rest/v1/content_translations?limit=1", url))
        .header("apikey", key).header("Authorization", format!("Bearer {}", key))
        .send().await.context("Failed to check schema")?;
    if !resp.status().is_success() { anyhow::bail!("Schema check failed"); }
    Ok(())
}

async fn fetch_posts(client: &Client, url: &str, key: &str, limit: usize) -> Result<Vec<Post>> {
    let resp = client.get(format!("{}/rest/v1/posts?select=id,post_name,post_description&is_active=eq.true&limit={}", url, limit))
        .header("apikey", key).header("Authorization", format!("Bearer {}", key))
        .send().await.context("Failed to fetch posts")?;
    if !resp.status().is_success() { anyhow::bail!("Failed to fetch posts"); }
    let posts: Vec<Post> = resp.json().await?;
    if posts.is_empty() { anyhow::bail!("No active posts found"); }
    Ok(posts)
}

async fn trigger_translation(client: &Client, url: &str, key: &str, post: &Post) -> Result<()> {
    let mut fields = vec![TranslationField { name: "title".to_string(), text: post.post_name.clone() }];
    if let Some(desc) = &post.post_description {
        if !desc.is_empty() { fields.push(TranslationField { name: "description".to_string(), text: desc.clone() }); }
    }
    let req = TranslateBatchRequest { content_type: "post".to_string(), content_id: post.id.to_string(), fields };
    client.post(format!("{}/functions/v1/localization/translate-batch", url))
        .header("Authorization", format!("Bearer {}", key))
        .json(&req).send().await?;
    Ok(())
}

async fn fetch_translations(client: &Client, url: &str, key: &str, posts: &[Post], locale: &str) -> Result<TranslationResponse> {
    let ids: Vec<String> = posts.iter().map(|p| p.id.to_string()).collect();
    let resp = client.post(format!("{}/functions/v1/localization/get-translations", url))
        .header("Authorization", format!("Bearer {}", key))
        .json(&json!({ "contentType": "post", "contentIds": ids, "locale": locale, "fields": ["title", "description"] }))
        .send().await.context("Failed to fetch translations")?;
    if !resp.status().is_success() { anyhow::bail!("Failed to fetch translations"); }
    resp.json().await.context("Failed to parse translations")
}
