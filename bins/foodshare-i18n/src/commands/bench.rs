//! Benchmark command - performance testing for translation endpoints

use crate::api::ApiClient;
use anyhow::Result;
use owo_colors::OwoColorize;
use serde::Serialize;
use std::time::Duration;

/// JSON output for benchmark
#[derive(Debug, Serialize)]
struct JsonBenchOutput {
    locale: String,
    requests: usize,
    bff: BenchStats,
    direct: BenchStats,
    comparison: BenchComparison,
}

#[derive(Debug, Serialize)]
struct BenchStats {
    min_ms: u64,
    max_ms: u64,
    avg_ms: f64,
    p50_ms: u64,
    p95_ms: u64,
    p99_ms: u64,
    success_rate: f64,
}

#[derive(Debug, Serialize)]
struct BenchComparison {
    bff_faster: bool,
    difference_ms: f64,
    difference_percent: f64,
}

/// Run benchmark
pub async fn run(count: usize, locale: &str, format: &str) -> Result<()> {
    let client = ApiClient::new()?;

    if format == "json" {
        return run_json(&client, count, locale).await;
    }

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!(
        "  {}",
        format!("⚡ Benchmark: {} requests for '{}'", count, locale)
            .blue()
            .bold()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!();

    // Benchmark BFF endpoint
    println!("{}", "Benchmarking BFF endpoint...".yellow());
    let bff_times = benchmark_bff(&client, locale, count).await;
    let bff_stats = calculate_stats(&bff_times);
    print_stats("BFF", &bff_stats);

    println!();

    // Benchmark direct endpoint
    println!("{}", "Benchmarking direct endpoint...".yellow());
    let direct_times = benchmark_direct(&client, locale, count).await;
    let direct_stats = calculate_stats(&direct_times);
    print_stats("Direct", &direct_stats);

    println!();

    // Comparison
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!("  {}", "📊 Comparison".blue().bold());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .blue()
    );
    println!();

    let diff = bff_stats.avg - direct_stats.avg;
    let diff_percent = if direct_stats.avg > 0.0 {
        (diff / direct_stats.avg) * 100.0
    } else {
        0.0
    };

    if diff < 0.0 {
        println!(
            "  {} BFF is {:.1}ms ({:.1}%) faster on average",
            "✓".green(),
            diff.abs(),
            diff_percent.abs()
        );
    } else if diff > 0.0 {
        println!(
            "  {} Direct is {:.1}ms ({:.1}%) faster on average",
            "✓".green(),
            diff.abs(),
            diff_percent.abs()
        );
    } else {
        println!("  {} Both endpoints have similar performance", "=".cyan());
    }

    println!();
    Ok(())
}

async fn benchmark_bff(client: &ApiClient, locale: &str, count: usize) -> Vec<Duration> {
    let mut times = Vec::with_capacity(count);

    for i in 0..count {
        print!("\r  Request {}/{}", i + 1, count);
        std::io::Write::flush(&mut std::io::stdout()).ok();

        if let Ok((_, elapsed)) = client.fetch_bff_translations(locale, None).await {
            times.push(elapsed);
        }
    }
    println!();

    times
}

async fn benchmark_direct(client: &ApiClient, locale: &str, count: usize) -> Vec<Duration> {
    let mut times = Vec::with_capacity(count);

    for i in 0..count {
        print!("\r  Request {}/{}", i + 1, count);
        std::io::Write::flush(&mut std::io::stdout()).ok();

        if let Ok((_, elapsed)) = client.fetch_direct_translations(locale).await {
            times.push(elapsed);
        }
    }
    println!();

    times
}

struct Stats {
    min: u64,
    max: u64,
    avg: f64,
    p50: u64,
    p95: u64,
    p99: u64,
    success_rate: f64,
    count: usize,
}

fn calculate_stats(times: &[Duration]) -> Stats {
    if times.is_empty() {
        return Stats {
            min: 0,
            max: 0,
            avg: 0.0,
            p50: 0,
            p95: 0,
            p99: 0,
            success_rate: 0.0,
            count: 0,
        };
    }

    let mut ms_times: Vec<u64> = times.iter().map(|d| d.as_millis() as u64).collect();
    ms_times.sort();

    let sum: u64 = ms_times.iter().sum();
    let avg = sum as f64 / ms_times.len() as f64;

    let p50_idx = (ms_times.len() as f64 * 0.50) as usize;
    let p95_idx = (ms_times.len() as f64 * 0.95) as usize;
    let p99_idx = (ms_times.len() as f64 * 0.99) as usize;

    Stats {
        min: *ms_times.first().unwrap_or(&0),
        max: *ms_times.last().unwrap_or(&0),
        avg,
        p50: ms_times.get(p50_idx).copied().unwrap_or(0),
        p95: ms_times.get(p95_idx.min(ms_times.len() - 1)).copied().unwrap_or(0),
        p99: ms_times.get(p99_idx.min(ms_times.len() - 1)).copied().unwrap_or(0),
        success_rate: 100.0,
        count: ms_times.len(),
    }
}

fn print_stats(name: &str, stats: &Stats) {
    println!();
    println!("  {} ({} requests):", name.cyan(), stats.count);
    println!(
        "    Min:  {}ms",
        stats.min.to_string().green()
    );
    println!(
        "    Max:  {}ms",
        stats.max.to_string().yellow()
    );
    println!(
        "    Avg:  {:.1}ms",
        stats.avg
    );
    println!(
        "    P50:  {}ms",
        stats.p50
    );
    println!(
        "    P95:  {}ms",
        stats.p95.to_string().yellow()
    );
    println!(
        "    P99:  {}ms",
        stats.p99.to_string().red()
    );
}

async fn run_json(client: &ApiClient, count: usize, locale: &str) -> Result<()> {
    // Benchmark BFF
    let bff_times = benchmark_bff_silent(client, locale, count).await;
    let bff_stats = calculate_stats(&bff_times);

    // Benchmark direct
    let direct_times = benchmark_direct_silent(client, locale, count).await;
    let direct_stats = calculate_stats(&direct_times);

    // Calculate comparison
    let diff = bff_stats.avg - direct_stats.avg;
    let diff_percent = if direct_stats.avg > 0.0 {
        (diff / direct_stats.avg) * 100.0
    } else {
        0.0
    };

    let output = JsonBenchOutput {
        locale: locale.to_string(),
        requests: count,
        bff: BenchStats {
            min_ms: bff_stats.min,
            max_ms: bff_stats.max,
            avg_ms: bff_stats.avg,
            p50_ms: bff_stats.p50,
            p95_ms: bff_stats.p95,
            p99_ms: bff_stats.p99,
            success_rate: bff_stats.success_rate,
        },
        direct: BenchStats {
            min_ms: direct_stats.min,
            max_ms: direct_stats.max,
            avg_ms: direct_stats.avg,
            p50_ms: direct_stats.p50,
            p95_ms: direct_stats.p95,
            p99_ms: direct_stats.p99,
            success_rate: direct_stats.success_rate,
        },
        comparison: BenchComparison {
            bff_faster: diff < 0.0,
            difference_ms: diff.abs(),
            difference_percent: diff_percent.abs(),
        },
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

async fn benchmark_bff_silent(client: &ApiClient, locale: &str, count: usize) -> Vec<Duration> {
    let mut times = Vec::with_capacity(count);
    for _ in 0..count {
        if let Ok((_, elapsed)) = client.fetch_bff_translations(locale, None).await {
            times.push(elapsed);
        }
    }
    times
}

async fn benchmark_direct_silent(client: &ApiClient, locale: &str, count: usize) -> Vec<Duration> {
    let mut times = Vec::with_capacity(count);
    for _ in 0..count {
        if let Ok((_, elapsed)) = client.fetch_direct_translations(locale).await {
            times.push(elapsed);
        }
    }
    times
}
