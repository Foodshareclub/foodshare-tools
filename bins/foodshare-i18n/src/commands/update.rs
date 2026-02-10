//! Update translations commands
//!
//! Replaces the shell scripts:
//! - fill_missing_translations.sh
//! - update_translations.sh
//! - update_ios_translations.sh

use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Response from the update-translations endpoint
#[derive(Debug, Deserialize)]
struct UpdateResponse {
    success: bool,
    added: Option<usize>,
    total: Option<usize>,
    error: Option<String>,
}

/// Run the update command for a specific locale from a JSON file
pub async fn run_from_file(locale: &str, file_path: &str, format: &str) -> Result<()> {
    if format == "json" {
        return run_from_file_json(locale, file_path).await;
    }

    println!("{}", "Updating Translations from File".bold().cyan());
    println!("{}", "=".repeat(40).dimmed());
    println!();

    // Load translations from file
    let path = Path::new(file_path);
    if !path.exists() {
        anyhow::bail!("Translation file not found: {}", file_path);
    }

    println!("Locale: {}", locale.cyan());
    println!("File:   {}", file_path.cyan());
    println!();

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", file_path))?;

    let translations: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON from {}", file_path))?;

    // Count keys
    let key_count = count_keys(&translations);
    println!("Keys to update: {}", key_count);
    println!();

    // Send to API
    let result = update_locale(locale, &translations).await?;

    if result.success {
        println!(
            "{} {} updated successfully",
            "✓".green(),
            locale.green().bold()
        );
        if let Some(added) = result.added {
            println!("   Added: {} keys", added);
        }
        if let Some(total) = result.total {
            println!("   Total: {} keys", total);
        }
    } else {
        println!("{} {} failed:", "✗".red(), locale.red().bold());
        if let Some(err) = result.error {
            println!("   {}", err);
        }
    }

    Ok(())
}

/// Run from file in JSON mode
async fn run_from_file_json(locale: &str, file_path: &str) -> Result<()> {
    let path = Path::new(file_path);
    if !path.exists() {
        let output = serde_json::json!({
            "success": false,
            "error": format!("File not found: {}", file_path)
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let content = std::fs::read_to_string(path)?;
    let translations: serde_json::Value = serde_json::from_str(&content)?;

    let result = update_locale(locale, &translations).await?;

    let output = serde_json::json!({
        "success": result.success,
        "locale": locale,
        "file": file_path,
        "added": result.added,
        "total": result.total,
        "error": result.error
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Run the update command for multiple locales from inline data
pub async fn run_batch(locales: &[String], format: &str) -> Result<()> {
    if format == "json" {
        return run_batch_json(locales).await;
    }

    println!("{}", "Batch Update Translations".bold().cyan());
    println!("{}", "=".repeat(40).dimmed());
    println!();

    let mut success_count = 0;
    let mut fail_count = 0;

    for locale in locales {
        let translations = get_preset_translations(locale);

        if translations.is_none() {
            println!(
                "  {} {} - no preset translations available",
                "⚠".yellow(),
                locale
            );
            continue;
        }

        let translations = translations.unwrap();
        print!("  Updating {}...", locale.cyan());

        let result = update_locale(locale, &translations).await;

        match result {
            Ok(resp) if resp.success => {
                println!(
                    " {} (added: {})",
                    "✓".green(),
                    resp.added.unwrap_or(0)
                );
                success_count += 1;
            }
            Ok(resp) => {
                println!(" {} {}", "✗".red(), resp.error.unwrap_or_default());
                fail_count += 1;
            }
            Err(e) => {
                println!(" {} {}", "✗".red(), e);
                fail_count += 1;
            }
        }
    }

    println!();
    println!(
        "Summary: {} succeeded, {} failed",
        success_count.to_string().green(),
        fail_count.to_string().red()
    );

    Ok(())
}

/// Run batch in JSON mode
async fn run_batch_json(locales: &[String]) -> Result<()> {
    let mut results = Vec::new();

    for locale in locales {
        let translations = get_preset_translations(locale);

        let result = if let Some(translations) = translations {
            match update_locale(locale, &translations).await {
                Ok(resp) => serde_json::json!({
                    "locale": locale,
                    "success": resp.success,
                    "added": resp.added,
                    "total": resp.total,
                    "error": resp.error
                }),
                Err(e) => serde_json::json!({
                    "locale": locale,
                    "success": false,
                    "error": e.to_string()
                }),
            }
        } else {
            serde_json::json!({
                "locale": locale,
                "success": false,
                "error": "No preset translations available"
            })
        };

        results.push(result);
    }

    let output = serde_json::json!({
        "results": results,
        "total": locales.len(),
        "succeeded": results.iter().filter(|r| r["success"].as_bool().unwrap_or(false)).count()
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Send translations to the API
async fn update_locale(locale: &str, translations: &serde_json::Value) -> Result<UpdateResponse> {
    let base_url = std::env::var("SUPABASE_URL")
        .unwrap_or_else(|_| "https://api.foodshare.club".to_string());

    let service_key = std::env::var("SUPABASE_SERVICE_ROLE_KEY")
        .or_else(|_| std::env::var("SUPABASE_ANON_KEY"))
        .context("SUPABASE_SERVICE_ROLE_KEY or SUPABASE_ANON_KEY must be set")?;

    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "locale": locale,
        "translations": translations
    });

    let response = client
        .post(format!("{}/functions/v1/update-translations", base_url))
        .header("Authorization", format!("Bearer {}", service_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .context("Failed to connect to update-translations endpoint")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("HTTP {}: {}", status, body);
    }

    response
        .json::<UpdateResponse>()
        .await
        .context("Failed to parse response")
}

/// Count keys in a nested JSON object
fn count_keys(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(map) => {
            let mut count = 0;
            for v in map.values() {
                if v.is_object() {
                    count += count_keys(v);
                } else {
                    count += 1;
                }
            }
            count
        }
        _ => 0,
    }
}

/// Get preset translations for common locales
/// These are hardcoded translations for common missing keys
fn get_preset_translations(locale: &str) -> Option<serde_json::Value> {
    // Common keys that are often missing
    let common_keys: HashMap<&str, HashMap<&str, &str>> = [
        (
            "de",
            [
                ("ab_testing", "A/B-Tests"),
                ("about", "Über"),
                ("account", "Konto"),
                ("analytics", "Analytik"),
                ("challenges", "Herausforderungen"),
                ("dashboard", "Dashboard"),
                ("explore", "Entdecken"),
                ("listings", "Angebote"),
                ("overview", "Übersicht"),
                ("reports", "Berichte"),
                ("send", "Senden"),
                ("statistics", "Statistiken"),
                ("users", "Benutzer"),
                ("yes", "Ja"),
            ]
            .into_iter()
            .collect(),
        ),
        (
            "es",
            [
                ("ab_testing", "Pruebas A/B"),
                ("about", "Acerca de"),
                ("account", "Cuenta"),
                ("analytics", "Analítica"),
                ("challenges", "Desafíos"),
                ("dashboard", "Panel"),
                ("explore", "Explorar"),
                ("listings", "Anuncios"),
                ("overview", "Resumen"),
                ("reports", "Informes"),
                ("send", "Enviar"),
                ("statistics", "Estadísticas"),
                ("users", "Usuarios"),
                ("yes", "Sí"),
            ]
            .into_iter()
            .collect(),
        ),
        (
            "fr",
            [
                ("ab_testing", "Tests A/B"),
                ("about", "À propos"),
                ("account", "Compte"),
                ("analytics", "Analytique"),
                ("challenges", "Défis"),
                ("dashboard", "Tableau de bord"),
                ("explore", "Explorer"),
                ("listings", "Annonces"),
                ("overview", "Aperçu"),
                ("reports", "Rapports"),
                ("send", "Envoyer"),
                ("statistics", "Statistiques"),
                ("users", "Utilisateurs"),
                ("yes", "Oui"),
            ]
            .into_iter()
            .collect(),
        ),
        (
            "ru",
            [
                ("ab_testing", "A/B тестирование"),
                ("about", "О приложении"),
                ("account", "Аккаунт"),
                ("analytics", "Аналитика"),
                ("challenges", "Вызовы"),
                ("dashboard", "Панель"),
                ("explore", "Обзор"),
                ("listings", "Объявления"),
                ("overview", "Обзор"),
                ("reports", "Отчёты"),
                ("send", "Отправить"),
                ("statistics", "Статистика"),
                ("users", "Пользователи"),
                ("yes", "Да"),
            ]
            .into_iter()
            .collect(),
        ),
        (
            "uk",
            [
                ("ab_testing", "A/B тестування"),
                ("about", "Про додаток"),
                ("account", "Обліковий запис"),
                ("analytics", "Аналітика"),
                ("challenges", "Виклики"),
                ("dashboard", "Панель"),
                ("explore", "Огляд"),
                ("listings", "Оголошення"),
                ("overview", "Огляд"),
                ("reports", "Звіти"),
                ("send", "Надіслати"),
                ("statistics", "Статистика"),
                ("users", "Користувачі"),
                ("yes", "Так"),
            ]
            .into_iter()
            .collect(),
        ),
        (
            "zh",
            [
                ("ab_testing", "A/B测试"),
                ("about", "关于"),
                ("account", "账户"),
                ("analytics", "分析"),
                ("challenges", "挑战"),
                ("dashboard", "仪表板"),
                ("explore", "探索"),
                ("listings", "列表"),
                ("overview", "概览"),
                ("reports", "报告"),
                ("send", "发送"),
                ("statistics", "统计"),
                ("users", "用户"),
                ("yes", "是"),
            ]
            .into_iter()
            .collect(),
        ),
        (
            "ja",
            [
                ("ab_testing", "A/Bテスト"),
                ("about", "について"),
                ("account", "アカウント"),
                ("analytics", "分析"),
                ("challenges", "チャレンジ"),
                ("dashboard", "ダッシュボード"),
                ("explore", "探索"),
                ("listings", "リスト"),
                ("overview", "概要"),
                ("reports", "レポート"),
                ("send", "送信"),
                ("statistics", "統計"),
                ("users", "ユーザー"),
                ("yes", "はい"),
            ]
            .into_iter()
            .collect(),
        ),
        (
            "ko",
            [
                ("ab_testing", "A/B 테스트"),
                ("about", "정보"),
                ("account", "계정"),
                ("analytics", "분석"),
                ("challenges", "챌린지"),
                ("dashboard", "대시보드"),
                ("explore", "탐색"),
                ("listings", "목록"),
                ("overview", "개요"),
                ("reports", "보고서"),
                ("send", "보내기"),
                ("statistics", "통계"),
                ("users", "사용자"),
                ("yes", "예"),
            ]
            .into_iter()
            .collect(),
        ),
    ]
    .into_iter()
    .collect();

    common_keys.get(locale).map(|keys| {
        let mut map = serde_json::Map::new();
        for (k, v) in keys {
            map.insert(k.to_string(), serde_json::Value::String(v.to_string()));
        }
        serde_json::Value::Object(map)
    })
}
