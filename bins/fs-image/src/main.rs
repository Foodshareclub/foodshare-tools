//! fs-image: CLI tool for image processing and optimization.

use clap::{Parser, Subcommand};
use foodshare_image::{detect_format, extract_metadata, calculate_target_width};
use std::path::PathBuf;
use walkdir::WalkDir;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser)]
#[command(name = "fs-image")]
#[command(about = "Image processing and optimization CLI")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Detect image format from file
    Detect {
        /// Path to image file
        path: PathBuf,
    },
    /// Extract metadata from image
    Metadata {
        /// Path to image file
        path: PathBuf,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Analyze images in a directory
    Analyze {
        /// Directory to analyze
        path: PathBuf,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Calculate recommended resize for image
    Recommend {
        /// Path to image file
        path: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Detect { path } => {
            let data = std::fs::read(&path)?;
            match detect_format(&data) {
                Ok(format) => {
                    println!("Format: {:?}", format);
                    println!("MIME: {}", format.mime_type());
                    println!("Extensions: {:?}", format.extensions());
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Metadata { path, json } => {
            let data = std::fs::read(&path)?;
            match extract_metadata(&data) {
                Some(meta) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&meta)?);
                    } else {
                        println!("Format: {:?}", meta.format);
                        println!("Dimensions: {}x{}", meta.width, meta.height);
                        println!("Aspect Ratio: {:.2}", meta.aspect_ratio());
                        println!("Size: {} bytes", meta.size_bytes);
                        println!("Orientation: {}", if meta.is_landscape() { "Landscape" } else if meta.is_portrait() { "Portrait" } else { "Square" });
                    }
                }
                None => {
                    eprintln!("Could not extract metadata");
                    std::process::exit(1);
                }
            }
        }

        Commands::Analyze { path, json } => {
            let mut results = Vec::new();
            let entries: Vec<_> = WalkDir::new(&path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .collect();

            let pb = ProgressBar::new(entries.len() as u64);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")?
                .progress_chars("#>-"));

            for entry in entries {
                pb.inc(1);
                if let Ok(data) = std::fs::read(entry.path()) {
                    if let Some(meta) = extract_metadata(&data) {
                        let target_width = calculate_target_width(meta.size_bytes, meta.width, meta.height);
                        results.push(serde_json::json!({
                            "path": entry.path().to_string_lossy(),
                            "format": meta.format,
                            "width": meta.width,
                            "height": meta.height,
                            "size_bytes": meta.size_bytes,
                            "needs_resize": target_width > 0,
                            "target_width": target_width,
                        }));
                    }
                }
            }
            pb.finish_with_message("Done");

            if json {
                println!("{}", serde_json::to_string_pretty(&results)?);
            } else {
                println!("\nFound {} images", results.len());
                let needs_resize: Vec<_> = results.iter()
                    .filter(|r| r["needs_resize"].as_bool().unwrap_or(false))
                    .collect();
                println!("{} images need resizing", needs_resize.len());
            }
        }

        Commands::Recommend { path } => {
            let data = std::fs::read(&path)?;
            match extract_metadata(&data) {
                Some(meta) => {
                    let target = calculate_target_width(meta.size_bytes, meta.width, meta.height);
                    if target > 0 {
                        println!("Recommended resize: {} -> {} pixels wide", meta.width, target);
                        let (_, new_height) = foodshare_image::smart_width::calculate_dimensions(
                            meta.width, meta.height, target
                        );
                        println!("New dimensions: {}x{}", target, new_height);
                    } else {
                        println!("No resize needed - image is already optimized");
                    }
                }
                None => {
                    eprintln!("Could not analyze image");
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
