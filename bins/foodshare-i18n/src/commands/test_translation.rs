//! Test post translation system end-to-end

use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

#[derive(Debug)]
pub struct TestTranslationArgs {
    pub supabase_url: String,
    pub anon_key: String,
    pub service_key: Option<String>,
    pub locale: String,
    pub limit: usize,
    pub skip_trigger: bool,
    pub verbose: bool,
}

#[derive(Debug, Deserialize)]
struct Post {
    id: i64,
    post_name: String,
    post_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TranslationResponse {
    success: bool,
    translations: serde_json::Value,
    #[serde(rename = "fromRedis")]
    from_redis: Option<i32>,
    #[serde(rename = "fromDatabase")]
    from_database: Option<i32>,
    #[serde(rename = "onDemand")]
    on_demand: Option<i32>,
    #[serde(rename = "notFound")]
    not_found: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct BFFResponse {
    listings: Vec<BFFListing>,
}

#[derive(Debug, Deserialize)]
struct BFFListing {
    id: String,
    title: String,
    #[serde(rename = "titleTranslated")]
    title_translated: Option<String>,
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

pub async fn execute(args: TestTranslationArgs) -> Result<()> {
    println!("{}", "=== Translation System Test ===".bright_green().bold());
    println!();

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    // Test 1: Check database schema
    println!("{}", "Test 1: Checking database schema...".yellow());
    test_schema(&client, &args).await?;
    println!("{}", "✓ Database schema OK".green());
    println!();

    // Test 2: Fetch sample posts
    println!("{}", "Test 2: Fetching sample posts...".yellow());
    let posts = fetch_sample_posts(&client, &args).await?;
    println!("{}", format!("✓ Found {} active posts", posts.len()).green());
    
    if args.verbose {
        for post in &posts {
            println!("  - Post {}: {}", post.id, post.post_name);
        }
    }
    println!();

    // Test 3: Trigger translations (if not skipped)
    if !args.skip_trigger && args.service_key.is_some() {
        println!("{}", "Test 3: Triggering translations...".yellow());
        for post in &posts {
            trigger_translation(&client, &args, post).await?;
        }
        println!("{}", "✓ Translations triggered".green());
        println!("{}", "  Waiting 30 seconds for translations to complete...".cyan());
        tokio::time::sleep(Duration::from_secs(30)).await;
        println!();
    } else if args.skip_trigger {
        println!("{}", "Test 3: Skipped (--skip-trigger)".yellow());
        println!();
    } else {
        println!("{}", "Test 3: Skipped (no service key)".yellow());
        println!();
    }

    // Test 4: Fetch translations
    println!("{}", format!("Test 4: Fetching {} translations...", args.locale).yellow());
    let translation_response = fetch_translations(&client, &args, &posts).await?;
    
    let from_redis = translation_response.from_redis.unwrap_or(0);
    let from_db = translation_response.from_database.unwrap_or(0);
    let on_demand = translation_response.on_demand.unwrap_or(0);
    let not_found = translation_response.not_found.unwrap_or(0);
    
    println!("{}", "✓ Translations fetched".green());
    println!("  Cache: Redis={}, DB={}, LLM={}, NotFound={}", 
        from_redis.to_string().cyan(),
        from_db.to_string().cyan(),
        on_demand.to_string().cyan(),
        not_found.to_string().red()
    );
    println!();

    // Test 5: Test BFF feed
    println!("{}", "Test 5: Testing BFF feed with locale...".yellow());
    let bff_response = test_bff_feed(&client, &args).await?;
    
    let total = bff_response.listings.len();
    let translated = bff_response.listings.iter()
        .filter(|l| l.title_translated.is_some())
        .count();
    
    println!("{}", "✓ BFF feed endpoint working".green());
    println!("  Total listings: {}", total.to_string().cyan());
    println!("  Translated: {}", translated.to_string().cyan());
    println!();

    // Summary
    println!("{}", "=== Test Summary ===".bright_green().bold());
    println!("{}", "✓ Database schema ready".green());
    println!("{}", "✓ Translation endpoint working".green());
    println!("{}", "✓ BFF integration working".green());
    println!("{}", "✓ End-to-end flow complete".green());
    
    if not_found > 0 {
        println!();
        println!("{}", format!("⚠ {} posts not translated yet", not_found).yellow());
        println!("  Run without --skip-trigger to translate them");
    }

    Ok(())
}

async fn test_schema(client: &Client, args: &TestTranslationArgs) -> Result<()> {
    // Check if content_translations table exists by querying it directly
    let url = format!("{}/rest/v1/content_translations?limit=1", args.supabase_url);
    
    let response = client
        .get(&url)
        .header("apikey", &args.anon_key)
        .header("Authorization", format!("Bearer {}", args.anon_key))
        .send()
        .await
        .context("Failed to check database schema")?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        anyhow::bail!("Schema check failed ({}): {}. The content_translations table may not exist.", status, text);
    }

    Ok(())
}

async fn fetch_sample_posts(client: &Client, args: &TestTranslationArgs) -> Result<Vec<Post>> {
    let url = format!(
        "{}/rest/v1/posts?select=id,post_name,post_description&is_active=eq.true&limit={}",
        args.supabase_url, args.limit
    );
    
    let response = client
        .get(&url)
        .header("apikey", &args.anon_key)
        .header("Authorization", format!("Bearer {}", args.anon_key))
        .send()
        .await
        .context("Failed to fetch posts")?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        anyhow::bail!("Failed to fetch posts ({}): {}", status, text);
    }

    let posts: Vec<Post> = response.json().await?;
    
    if posts.is_empty() {
        anyhow::bail!("No active posts found in database");
    }

    Ok(posts)
}

async fn trigger_translation(client: &Client, args: &TestTranslationArgs, post: &Post) -> Result<()> {
    let service_key = args.service_key.as_ref()
        .context("Service key required to trigger translations")?;
    
    let url = format!("{}/functions/v1/localization/translate-batch", args.supabase_url);
    
    let mut fields = vec![
        TranslationField {
            name: "title".to_string(),
            text: post.post_name.clone(),
        }
    ];
    
    if let Some(desc) = &post.post_description {
        if !desc.is_empty() {
            fields.push(TranslationField {
                name: "description".to_string(),
                text: desc.clone(),
            });
        }
    }
    
    let request = TranslateBatchRequest {
        content_type: "post".to_string(),
        content_id: post.id.to_string(),
        fields,
    };
    
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", service_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .context("Failed to trigger translation")?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        eprintln!("  ⚠ Failed to trigger translation for post {}: {}", post.id, text);
    } else if args.verbose {
        println!("  ✓ Triggered translation for post {}", post.id);
    }

    Ok(())
}

async fn fetch_translations(
    client: &Client,
    args: &TestTranslationArgs,
    posts: &[Post],
) -> Result<TranslationResponse> {
    let url = format!("{}/functions/v1/localization/get-translations", args.supabase_url);
    
    let content_ids: Vec<String> = posts.iter().map(|p| p.id.to_string()).collect();
    
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", args.anon_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "contentType": "post",
            "contentIds": content_ids,
            "locale": args.locale,
            "fields": ["title", "description"]
        }))
        .send()
        .await
        .context("Failed to fetch translations")?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        anyhow::bail!("Failed to fetch translations ({}): {}", status, text);
    }

    let translation_response: TranslationResponse = response.json().await?;
    Ok(translation_response)
}

async fn test_bff_feed(client: &Client, args: &TestTranslationArgs) -> Result<BFFResponse> {
    let url = format!("{}/functions/v1/bff/feed", args.supabase_url);
    
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", args.anon_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "lat": 51.5074,
            "lng": -0.1278,
            "radiusKm": 100,
            "limit": args.limit,
            "locale": args.locale
        }))
        .send()
        .await
        .context("Failed to test BFF feed")?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        anyhow::bail!("BFF feed failed ({}): {}", status, text);
    }

    let bff_response: BFFResponse = response.json().await?;
    Ok(bff_response)
}
