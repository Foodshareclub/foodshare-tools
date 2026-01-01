//! Bundle size analysis
//!
//! Analyzes Next.js build output for bundle sizes.

use anyhow::Result;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Bundle info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleInfo {
    pub name: String,
    pub size: u64,
    pub gzip_size: Option<u64>,
}

/// Bundle analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleAnalysis {
    pub total_size: u64,
    pub total_gzip: u64,
    pub bundles: Vec<BundleInfo>,
    pub largest_bundle: Option<String>,
}

/// Analyze Next.js build output
pub fn analyze_nextjs_build(build_dir: &Path) -> Result<BundleAnalysis> {
    let mut bundles = Vec::new();
    let mut total_size = 0u64;
    let total_gzip = 0u64;

    // Look for .next/static/chunks
    let chunks_dir = build_dir.join(".next/static/chunks");
    if chunks_dir.exists() {
        for entry in std::fs::read_dir(&chunks_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |e| e == "js") {
                let size = std::fs::metadata(&path)?.len();
                total_size += size;

                bundles.push(BundleInfo {
                    name: path.file_name().unwrap().to_string_lossy().to_string(),
                    size,
                    gzip_size: None, // Would need to actually gzip to get this
                });
            }
        }
    }

    // Sort by size descending
    bundles.sort_by(|a, b| b.size.cmp(&a.size));

    let largest_bundle = bundles.first().map(|b| b.name.clone());

    Ok(BundleAnalysis {
        total_size,
        total_gzip,
        bundles,
        largest_bundle,
    })
}

/// Format size for display
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Print bundle analysis
pub fn print_analysis(analysis: &BundleAnalysis, threshold_kb: Option<u64>) {
    println!("{}", "Bundle Size Analysis".bold());
    println!();
    println!("Total size: {}", format_size(analysis.total_size).cyan());
    println!();

    if !analysis.bundles.is_empty() {
        println!("{}", "Top bundles:".bold());
        for (i, bundle) in analysis.bundles.iter().take(10).enumerate() {
            let size_str = format_size(bundle.size);
            let warning = threshold_kb
                .map(|t| bundle.size > t * 1024)
                .unwrap_or(false);

            if warning {
                println!(
                    "  {}. {} - {} {}",
                    i + 1,
                    bundle.name,
                    size_str.red(),
                    "⚠".yellow()
                );
            } else {
                println!("  {}. {} - {}", i + 1, bundle.name, size_str);
            }
        }
    }

    if let Some(threshold) = threshold_kb {
        let over_threshold: Vec<_> = analysis
            .bundles
            .iter()
            .filter(|b| b.size > threshold * 1024)
            .collect();

        if !over_threshold.is_empty() {
            println!();
            eprintln!(
                "{} {} bundle(s) exceed {} KB threshold",
                "⚠".yellow(),
                over_threshold.len(),
                threshold
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(500), "500 B");
    }

    #[test]
    fn test_format_size_kb() {
        assert_eq!(format_size(2048), "2.00 KB");
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size(2 * 1024 * 1024), "2.00 MB");
    }

    #[test]
    fn test_bundle_info_struct() {
        let bundle = BundleInfo {
            name: "main.js".to_string(),
            size: 1024,
            gzip_size: Some(512),
        };
        assert_eq!(bundle.name, "main.js");
    }
}
