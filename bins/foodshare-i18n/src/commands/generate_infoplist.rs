//! Generate InfoPlist.strings - Production-grade iOS permission localization
//!
//! Features:
//! - Progress indication with locale-by-locale status
//! - Backup of existing files before overwriting
//! - Verification of written files
//! - Custom strings file support
//! - Diff preview mode

use crate::api::ApiClient;
use crate::config::get_locale_info;
use crate::types::{GenerateInfoPlistStringsResponse, InfoPlistStats, JsonGenerateInfoPlistOutput};
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;

/// Default English permission strings for Foodshare iOS
const DEFAULT_STRINGS: &[(&str, &str)] = &[
    (
        "NSCameraUsageDescription",
        "Foodshare needs camera access to photograph food items you want to share with your community.",
    ),
    (
        "NSFaceIDUsageDescription",
        "Foodshare uses Face ID to securely unlock the app and protect your account.",
    ),
    (
        "NSLocationAlwaysAndWhenInUseUsageDescription",
        "Foodshare uses your location to show nearby food, notify you of new items, and help others find your shared food.",
    ),
    (
        "NSLocationWhenInUseUsageDescription",
        "Foodshare uses your location to show food available near you.",
    ),
    (
        "NSMicrophoneUsageDescription",
        "Foodshare uses the microphone for voice messages and hands-free search.",
    ),
    (
        "NSPhotoLibraryUsageDescription",
        "Foodshare needs photo library access to select images of food you want to share.",
    ),
    (
        "NSSpeechRecognitionUsageDescription",
        "Foodshare uses speech recognition to help you search for food hands-free.",
    ),
];

/// Run the generate-infoplist command
pub async fn run(
    dry_run: bool,
    skip_cache: bool,
    strings_file: Option<&str>,
    format: &str,
) -> Result<()> {
    let client = ApiClient::new()?;

    // Load strings from file or use defaults
    let strings = match strings_file {
        Some(path) => load_strings_from_file(path)?,
        None => default_strings(),
    };

    if format == "json" {
        return run_json(&client, &strings, dry_run, skip_cache).await;
    }

    print_header();

    if dry_run {
        println!("  {} Dry-run mode - no files will be written", "ℹ".cyan());
        println!();
    }

    // Show what we're translating
    println!("  {} Input strings:", "→".dimmed());
    for (key, value) in &strings {
        let truncated = truncate(value, 50);
        println!("    {} {}", key.dimmed(), truncated.italic());
    }
    println!();

    // Call API with progress indication
    print!("  {} Generating translations", "⋯".yellow());
    io::stdout().flush()?;

    let start = std::time::Instant::now();
    let response = client
        .generate_infoplist_strings(&strings, skip_cache)
        .await
        .context("API call failed")?;
    let duration = start.elapsed();

    // Clear progress line
    print!("\r");
    io::stdout().flush()?;

    if !response.success {
        println!(
            "  {} Generation failed",
            "✗".red()
        );
        for err in &response.errors {
            println!("    {} {}", "→".red(), err);
        }
        return Ok(());
    }

    // Show results
    print_stats(&response, duration);

    if dry_run {
        print_preview(&response);
        println!();
        println!(
            "  {} Run without {} to write files",
            "ℹ".cyan(),
            "--dry-run".bold()
        );
    } else {
        write_files(&response)?;
    }

    // Show any warnings
    if !response.errors.is_empty() {
        println!();
        println!("  {} Warnings:", "⚠".yellow());
        for err in &response.errors {
            println!("    {}", err.dimmed());
        }
    }

    println!();
    Ok(())
}

fn print_header() {
    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".blue()
    );
    println!("  {} Generate InfoPlist.strings", "📱".to_string().blue());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".blue()
    );
    println!();
}

fn print_stats(response: &GenerateInfoPlistStringsResponse, duration: std::time::Duration) {
    if let Some(stats) = &response.stats {
        println!("  {} Translation complete:", "✓".green());
        println!(
            "    ├─ Locales:     {} ({})",
            stats.total_locales.to_string().cyan(),
            format!("{} cached, {} translated", stats.from_cache, stats.translated_count).dimmed()
        );
        println!(
            "    ├─ Strings:     {} per locale",
            stats.total_strings.to_string().cyan()
        );
        if stats.failed_count > 0 {
            println!(
                "    ├─ Failed:      {} (using fallback)",
                stats.failed_count.to_string().yellow()
            );
        }
        println!(
            "    └─ Duration:    {:.1}s (API: {}ms)",
            duration.as_secs_f64(),
            stats.duration_ms
        );
    }
    println!();
}

fn print_preview(response: &GenerateInfoPlistStringsResponse) {
    let locales = get_locale_info();
    let preview_codes = ["ru", "es", "zh", "ar"];

    println!("  {} Preview:", "→".dimmed());

    for code in preview_codes {
        if let Some(translations) = response.locales.get(code) {
            let info = locales.iter().find(|l| l.code == code);
            let flag = info.map(|l| l.flag).unwrap_or("");
            let folder = response.lproj_folders.get(code).map(|s| s.as_str()).unwrap_or(code);

            println!();
            println!("    {} {} ({}.lproj):", flag, code.cyan(), folder);

            // Show first 2 translations
            for (key, value) in translations.iter().take(2) {
                let short_key = key.replace("NSUsageDescription", "");
                let truncated = truncate(value, 55);
                println!("      {} → {}", short_key.dimmed(), truncated);
            }

            if translations.len() > 2 {
                println!("      {} and {} more...", "...".dimmed(), translations.len() - 2);
            }
        }
    }
}

fn write_files(response: &GenerateInfoPlistStringsResponse) -> Result<()> {
    let resources_path = find_resources_path()?;

    println!("  {} Writing files to:", "→".dimmed());
    println!("    {}", resources_path.display().to_string().cyan());
    println!();

    let mut written = 0;
    let mut backed_up = 0;

    for (locale, content) in &response.files {
        let folder = response
            .lproj_folders
            .get(locale)
            .map(|s| s.as_str())
            .unwrap_or(locale);

        let lproj = resources_path.join(format!("{}.lproj", folder));
        let file_path = lproj.join("InfoPlist.strings");

        // Create directory if needed
        if !lproj.exists() {
            std::fs::create_dir_all(&lproj)?;
        }

        // Backup existing file
        if file_path.exists() {
            let backup = file_path.with_extension("strings.bak");
            std::fs::copy(&file_path, &backup)?;
            backed_up += 1;
        }

        // Write new content
        std::fs::write(&file_path, content)?;
        written += 1;

        // Verify write
        let verify = std::fs::read_to_string(&file_path)?;
        if verify != *content {
            println!("    {} {}.lproj - verification failed!", "⚠".yellow(), folder);
        } else {
            print!("    {} {}.lproj", "✓".green(), folder);
            if backed_up > 0 && file_path.with_extension("strings.bak").exists() {
                print!(" {}", "(backed up)".dimmed());
            }
            println!();
        }
    }

    println!();
    println!(
        "  {} Wrote {} files{}",
        "✓".green(),
        written.to_string().cyan(),
        if backed_up > 0 {
            format!(" ({} backed up)", backed_up)
        } else {
            String::new()
        }
    );

    Ok(())
}

fn find_resources_path() -> Result<PathBuf> {
    let candidates = [
        "FoodShare/Resources",
        "../foodshare-ios/FoodShare/Resources",
        "../../foodshare-ios/FoodShare/Resources",
        "../FoodShare/Resources",
    ];

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() && path.is_dir() {
            return path
                .canonicalize()
                .context("Failed to resolve resources path");
        }
    }

    // Try from current dir
    let cwd = std::env::current_dir()?;

    // In foodshare-ios
    let ios = cwd.join("FoodShare/Resources");
    if ios.exists() {
        return Ok(ios);
    }

    // In foodshare monorepo
    let mono = cwd.join("foodshare-ios/FoodShare/Resources");
    if mono.exists() {
        return Ok(mono);
    }

    anyhow::bail!(
        "Cannot find iOS Resources directory.\n\
         Expected: FoodShare/Resources\n\
         Run from: foodshare-ios/ or foodshare/ directory"
    )
}

fn default_strings() -> HashMap<String, String> {
    DEFAULT_STRINGS
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn load_strings_from_file(path: &str) -> Result<HashMap<String, String>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read strings file: {}", path))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Invalid JSON in strings file: {}", path))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

async fn run_json(
    client: &ApiClient,
    strings: &HashMap<String, String>,
    dry_run: bool,
    skip_cache: bool,
) -> Result<()> {
    let response = client.generate_infoplist_strings(strings, skip_cache).await?;

    let files_written = if response.success && !dry_run {
        let resources_path = find_resources_path()?;
        let mut written = Vec::new();

        for (locale, content) in &response.files {
            let folder = response
                .lproj_folders
                .get(locale)
                .map(|s| s.as_str())
                .unwrap_or(locale);

            let lproj = resources_path.join(format!("{}.lproj", folder));
            let file_path = lproj.join("InfoPlist.strings");

            if !lproj.exists() {
                std::fs::create_dir_all(&lproj)?;
            }

            std::fs::write(&file_path, content)?;
            written.push(file_path.display().to_string());
        }

        Some(written)
    } else {
        None
    };

    let output = JsonGenerateInfoPlistOutput {
        success: response.success,
        dry_run,
        locales_generated: response.locales.len(),
        strings_per_locale: strings.len(),
        files_written,
        stats: response.stats,
        errors: if response.errors.is_empty() {
            None
        } else {
            Some(response.errors)
        },
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
