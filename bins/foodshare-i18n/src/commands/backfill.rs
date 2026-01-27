//! Backfill translations for existing posts
//!
//! Replaces: populate-post-translations.ts
//!
//! Fetches all active posts from the database and triggers batch translation
//! to populate Redis cache and PostgreSQL with translations for all locales.

use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

/// Post data from the database
#[derive(Debug, Deserialize)]
struct Post {
    id: i64,
    post_name: String,
    post_description: Option<String>,
}

/// Translation field
#[derive(Debug, Serialize)]
struct TranslationField {
    name: String,
    text: String,
}

/// Batch translation request
#[derive(Debug, Serialize)]
struct BatchTranslateRequest {
    content_type: String,
    content_id: String,
    fields: Vec<TranslationField>,
}

/// Batch translation response
#[derive(Debug, Deserialize)]
struct BatchTranslateResponse {
    success: bool,
    total_translations: Option<i32>,
    queued: Option<i32>,
    error: Option<String>,
}

/// Supabase query response
#[derive(Debug, Deserialize)]
struct SupabaseResponse<T> {
    #[serde(flatten)]
    data: Option<Vec<T>>,
    error: Option<SupabaseError>,
}

#[derive(Debug, Deserialize)]
struct SupabaseError {
    message: String,
}

/// Run the backfill command
pub async fn run(
    batch_size: usize,
    delay_ms: u64,
    limit: Option<usize>,
    dry_run: bool,
    format: &str,
) -> Result<()> {
    if format == "json" {
        return run_json(batch_size, delay_ms, limit, dry_run).await;
    }

    println!("{}", "Post Translation Backfill".bold().cyan());
    println!("{}", "=".repeat(40).dimmed());
    println!();

    // Validate environment
    let base_url =
        std::env::var("SUPABASE_URL").context("SUPABASE_URL environment variable required")?;
    let service_key = std::env::var("SUPABASE_SERVICE_ROLE_KEY")
        .context("SUPABASE_SERVICE_ROLE_KEY environment variable required")?;

    println!("Configuration:");
    println!("  Batch size: {}", batch_size);
    println!("  Delay between batches: {}ms", delay_ms);
    if let Some(l) = limit {
        println!("  Limit: {} posts", l);
    }
    if dry_run {
        println!("  {} Dry run mode", "!".yellow());
    }
    println!();

    // Fetch posts
    println!("{}", "Fetching active posts...".bold());
    let posts = fetch_active_posts(&base_url, &service_key, limit).await?;

    if posts.is_empty() {
        println!("{}", "No posts to translate".yellow());
        return Ok(());
    }

    println!("  Found {} active posts", posts.len().to_string().green());
    println!();

    if dry_run {
        println!("{}", "Dry run - would process:".yellow());
        for post in posts.iter().take(5) {
            println!(
                "  Post {}: \"{}\"",
                post.id,
                truncate(&post.post_name, 40)
            );
        }
        if posts.len() > 5 {
            println!("  ... and {} more", posts.len() - 5);
        }
        return Ok(());
    }

    // Process posts in batches
    process_posts(&base_url, &service_key, &posts, batch_size, delay_ms).await?;

    println!();
    println!(
        "{}",
        "Backfill complete! Translations are processing in the background.".green().bold()
    );
    println!("  Check Redis and PostgreSQL for cached translations.");

    Ok(())
}

/// Run in JSON output mode
async fn run_json(
    batch_size: usize,
    delay_ms: u64,
    limit: Option<usize>,
    dry_run: bool,
) -> Result<()> {
    let base_url = std::env::var("SUPABASE_URL")?;
    let service_key = std::env::var("SUPABASE_SERVICE_ROLE_KEY")?;

    let posts = fetch_active_posts(&base_url, &service_key, limit).await?;

    if dry_run {
        let output = serde_json::json!({
            "success": true,
            "dry_run": true,
            "posts_found": posts.len(),
            "posts": posts.iter().take(10).map(|p| serde_json::json!({
                "id": p.id,
                "title": p.post_name,
                "has_description": p.post_description.is_some()
            })).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let (succeeded, failed) =
        process_posts_counted(&base_url, &service_key, &posts, batch_size, delay_ms).await?;

    let output = serde_json::json!({
        "success": true,
        "total": posts.len(),
        "succeeded": succeeded,
        "failed": failed,
        "estimated_translations": succeeded * 20 * 2
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Fetch active posts from the database
async fn fetch_active_posts(
    base_url: &str,
    service_key: &str,
    limit: Option<usize>,
) -> Result<Vec<Post>> {
    let client = reqwest::Client::new();

    let mut url = format!(
        "{}/rest/v1/posts?select=id,post_name,post_description&is_active=eq.true&post_name=not.is.null&order=created_at.desc",
        base_url
    );

    if let Some(l) = limit {
        url.push_str(&format!("&limit={}", l));
    }

    let response = client
        .get(&url)
        .header("apikey", service_key)
        .header("Authorization", format!("Bearer {}", service_key))
        .send()
        .await
        .context("Failed to connect to Supabase")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to fetch posts: HTTP {} - {}", status, body);
    }

    let posts: Vec<Post> = response.json().await.context("Failed to parse posts")?;

    Ok(posts)
}

/// Translate a single post
async fn translate_post(base_url: &str, service_key: &str, post: &Post) -> Result<i32> {
    let client = reqwest::Client::new();

    let mut fields = vec![TranslationField {
        name: "title".to_string(),
        text: post.post_name.clone(),
    }];

    if let Some(desc) = &post.post_description {
        if !desc.trim().is_empty() {
            fields.push(TranslationField {
                name: "description".to_string(),
                text: desc.clone(),
            });
        }
    }

    let request = BatchTranslateRequest {
        content_type: "post".to_string(),
        content_id: post.id.to_string(),
        fields,
    };

    let response = client
        .post(format!(
            "{}/functions/v1/localization/translate-batch",
            base_url
        ))
        .header("Authorization", format!("Bearer {}", service_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .context("Failed to send translation request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("HTTP {}: {}", status, body);
    }

    let result: BatchTranslateResponse = response.json().await.context("Failed to parse response")?;

    if let Some(err) = result.error {
        anyhow::bail!("{}", err);
    }

    Ok(result.total_translations.unwrap_or(0))
}

/// Process posts in batches
async fn process_posts(
    base_url: &str,
    service_key: &str,
    posts: &[Post],
    batch_size: usize,
    delay_ms: u64,
) -> Result<()> {
    println!(
        "Processing {} posts in batches of {}...",
        posts.len(),
        batch_size
    );
    println!();

    let mut processed = 0;
    let mut succeeded = 0;
    let mut failed = 0;

    let total_batches = (posts.len() + batch_size - 1) / batch_size;

    for (batch_idx, batch) in posts.chunks(batch_size).enumerate() {
        println!(
            "{}",
            format!("Batch {}/{}:", batch_idx + 1, total_batches).bold()
        );

        for post in batch {
            match translate_post(base_url, service_key, post).await {
                Ok(count) => {
                    println!(
                        "  {} Post {}: {} translations queued",
                        "✓".green(),
                        post.id,
                        count
                    );
                    succeeded += 1;
                }
                Err(e) => {
                    println!("  {} Post {}: {}", "✗".red(), post.id, e);
                    failed += 1;
                }
            }
        }

        processed += batch.len();
        println!(
            "Progress: {}/{} ({} succeeded, {} failed)",
            processed,
            posts.len(),
            succeeded.to_string().green(),
            failed.to_string().red()
        );

        // Delay between batches
        if batch_idx + 1 < total_batches {
            println!("  Waiting {}ms before next batch...", delay_ms);
            sleep(Duration::from_millis(delay_ms)).await;
        }

        println!();
    }

    println!("{}", "Summary:".bold());
    println!("  Total: {}", posts.len());
    println!("  Succeeded: {}", succeeded.to_string().green());
    println!("  Failed: {}", failed.to_string().red());
    println!(
        "  Estimated translations: {} (20 locales x 2 fields avg)",
        succeeded * 20 * 2
    );

    Ok(())
}

/// Process posts and return counts for JSON output
async fn process_posts_counted(
    base_url: &str,
    service_key: &str,
    posts: &[Post],
    batch_size: usize,
    delay_ms: u64,
) -> Result<(usize, usize)> {
    let mut succeeded = 0;
    let mut failed = 0;

    let total_batches = (posts.len() + batch_size - 1) / batch_size;

    for (batch_idx, batch) in posts.chunks(batch_size).enumerate() {
        for post in batch {
            match translate_post(base_url, service_key, post).await {
                Ok(_) => succeeded += 1,
                Err(_) => failed += 1,
            }
        }

        // Delay between batches
        if batch_idx + 1 < total_batches {
            sleep(Duration::from_millis(delay_ms)).await;
        }
    }

    Ok((succeeded, failed))
}

/// Truncate a string to a maximum length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
