//! fs-image: CLI tool for image processing and optimization.

use clap::{Parser, Subcommand};
use foodshare_image::{detect_format, extract_metadata, calculate_target_width};
use std::path::PathBuf;
use walkdir::WalkDir;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use image::GenericImageView;

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
    /// Remove alpha channel from images
    RemoveAlpha {
        /// Path to image file or directory
        path: PathBuf,
        /// Background color in hex format (e.g., ffffff for white)
        #[arg(long, default_value = "ffffff")]
        background: String,
        /// Overwrite original files
        #[arg(long)]
        overwrite: bool,
        /// Output directory (if not overwriting)
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
        /// Process directory recursively
        #[arg(long, short = 'r')]
        recursive: bool,
        /// Dry run - show what would be processed without making changes
        #[arg(long)]
        dry_run: bool,
        /// Only process images with specific dimensions (e.g., "1284x2778" for iPhone 6.9")
        #[arg(long)]
        filter_dimensions: Option<String>,
    },
    /// Resize images to specific dimensions
    Resize {
        /// Path to image file or directory
        path: PathBuf,
        /// Target width in pixels
        #[arg(long, short = 'w')]
        width: Option<u32>,
        /// Target height in pixels
        #[arg(long, short = 'h')]
        height: Option<u32>,
        /// Preset dimensions (e.g., "iphone-6.5-portrait" for 1242x2688)
        #[arg(long, short = 'p')]
        preset: Option<String>,
        /// Output directory (required)
        #[arg(long, short = 'o')]
        output: PathBuf,
        /// Process directory recursively
        #[arg(long, short = 'r')]
        recursive: bool,
        /// JPEG quality (1-100)
        #[arg(long, default_value = "90")]
        quality: u8,
        /// Dry run - show what would be processed without making changes
        #[arg(long)]
        dry_run: bool,
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

        Commands::RemoveAlpha { path, background, overwrite, output, recursive, dry_run, filter_dimensions } => {
            use foodshare_image::{process_image_file, has_alpha_channel, AlphaRemovalOptions};
            
            // Parse background color
            let bg_color = parse_hex_color(&background)?;
            
            // Parse filter dimensions if provided
            let dimension_filter = if let Some(ref dims) = filter_dimensions {
                let parts: Vec<&str> = dims.split('x').collect();
                if parts.len() != 2 {
                    anyhow::bail!("Invalid dimensions format. Use WIDTHxHEIGHT (e.g., 1284x2778)");
                }
                let w = parts[0].parse::<u32>()?;
                let h = parts[1].parse::<u32>()?;
                Some((w, h))
            } else {
                None
            };
            
            // Collect files to process
            let files: Vec<PathBuf> = if path.is_dir() {
                let walker = if recursive {
                    WalkDir::new(&path)
                } else {
                    WalkDir::new(&path).max_depth(1)
                };
                
                walker
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .filter(|e| is_image_file(e.path()))
                    .map(|e| e.path().to_path_buf())
                    .collect()
            } else if path.is_file() {
                vec![path.clone()]
            } else {
                anyhow::bail!("Path does not exist: {}", path.display());
            };

            if files.is_empty() {
                println!("No image files found");
                return Ok(());
            }

            println!("Found {} image file(s)", files.len());

            // Filter files that have alpha channels
            let files_with_alpha: Vec<PathBuf> = files
                .par_iter()
                .filter_map(|file_path| {
                    image::open(file_path).ok().and_then(|img| {
                        // Check dimension filter first
                        if let Some((filter_w, filter_h)) = dimension_filter {
                            let (w, h) = img.dimensions();
                            // Allow both portrait and landscape orientations
                            let matches = (w == filter_w && h == filter_h) || (w == filter_h && h == filter_w);
                            if !matches {
                                return None;
                            }
                        }
                        
                        if has_alpha_channel(&img) {
                            Some(file_path.clone())
                        } else {
                            None
                        }
                    })
                })
                .collect();

            println!("{} file(s) have alpha channels", files_with_alpha.len());

            if dry_run {
                println!("\nDry run - files that would be processed:");
                for file in &files_with_alpha {
                    println!("  {}", file.display());
                }
                return Ok(());
            }

            if files_with_alpha.is_empty() {
                println!("No files to process");
                return Ok(());
            }

            // Validate output configuration
            if !overwrite && output.is_none() {
                anyhow::bail!("Must specify either --overwrite or --output <directory>");
            }

            // Create output directory if needed
            if let Some(ref out_dir) = output {
                std::fs::create_dir_all(out_dir)?;
            }

            // Process files
            let pb = ProgressBar::new(files_with_alpha.len() as u64);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")?
                .progress_chars("#>-"));

            let options = AlphaRemovalOptions {
                background_color: bg_color,
                overwrite,
                output_format: None,
            };

            let results: Vec<Result<PathBuf, (PathBuf, foodshare_image::ImageError)>> = files_with_alpha
                .par_iter()
                .map(|file_path: &PathBuf| {
                    // Extract file name with fallback to full path for display
                    let file_name = file_path
                        .file_name()
                        .unwrap_or_else(|| file_path.as_os_str());

                    let output_path = if overwrite {
                        file_path.clone()
                    } else if let Some(ref out_dir) = output {
                        out_dir.join(file_name)
                    } else {
                        unreachable!()
                    };

                    let result = process_image_file(file_path, &output_path, &options);
                    pb.inc(1);

                    let display_name = file_name.to_string_lossy();
                    match result {
                        Ok(_) => {
                            pb.set_message(format!("✓ {display_name}"));
                            Ok(file_path.clone())
                        }
                        Err(e) => {
                            pb.set_message(format!("✗ {display_name}"));
                            Err((file_path.clone(), e))
                        }
                    }
                })
                .collect();

            pb.finish_with_message("Done");

            // Report results
            let successes: Vec<_> = results.iter().filter_map(|r| r.as_ref().ok()).collect();
            let failures: Vec<_> = results.iter().filter_map(|r| r.as_ref().err()).collect();

            println!("\n✓ Successfully processed {} file(s)", successes.len());
            
            if !failures.is_empty() {
                println!("✗ Failed to process {} file(s):", failures.len());
                for (path, err) in failures {
                    println!("  {}: {}", path.display(), err);
                }
            }
        }

        Commands::Resize { path, width, height, preset, output, recursive, quality, dry_run } => {
            use image::imageops::FilterType;
            
            // Determine target dimensions
            let (target_width, target_height) = if let Some(preset_name) = preset {
                match preset_name.as_str() {
                    "iphone-6.5-portrait" | "iphone65-portrait" => (1242, 2688),
                    "iphone-6.5-landscape" | "iphone65-landscape" => (2688, 1242),
                    "iphone-6.9-portrait" | "iphone69-portrait" => (1284, 2778),
                    "iphone-6.9-landscape" | "iphone69-landscape" => (2778, 1284),
                    "ipad-12.9-portrait" | "ipad129-portrait" => (2048, 2732),
                    "ipad-12.9-landscape" | "ipad129-landscape" => (2732, 2048),
                    _ => anyhow::bail!("Unknown preset: {}. Available: iphone-6.5-portrait, iphone-6.5-landscape, iphone-6.9-portrait, iphone-6.9-landscape, ipad-12.9-portrait, ipad-12.9-landscape", preset_name),
                }
            } else if let (Some(w), Some(h)) = (width, height) {
                (w, h)
            } else {
                anyhow::bail!("Must specify either --preset or both --width and --height");
            };

            // Collect files to process
            let files: Vec<PathBuf> = if path.is_dir() {
                let walker = if recursive {
                    WalkDir::new(&path)
                } else {
                    WalkDir::new(&path).max_depth(1)
                };
                
                walker
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .filter(|e| is_image_file(e.path()))
                    .map(|e| e.path().to_path_buf())
                    .collect()
            } else if path.is_file() {
                vec![path.clone()]
            } else {
                anyhow::bail!("Path does not exist: {}", path.display());
            };

            if files.is_empty() {
                println!("No image files found");
                return Ok(());
            }

            println!("Found {} image file(s)", files.len());
            println!("Target dimensions: {}x{}", target_width, target_height);

            if dry_run {
                println!("\nDry run - files that would be processed:");
                for file in &files {
                    if let Ok(img) = image::open(file) {
                        let (w, h) = img.dimensions();
                        println!("  {} ({}x{} -> {}x{})", 
                            file.display(), w, h, target_width, target_height);
                    }
                }
                return Ok(());
            }

            // Create output directory
            std::fs::create_dir_all(&output)?;

            // Process files
            let pb = ProgressBar::new(files.len() as u64);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")?
                .progress_chars("#>-"));

            let results: Vec<Result<PathBuf, (PathBuf, String)>> = files
                .par_iter()
                .map(|file_path: &PathBuf| {
                    // Extract file name with fallback to full path for display
                    let file_name = file_path
                        .file_name()
                        .unwrap_or_else(|| file_path.as_os_str());
                    let output_path = output.join(file_name);

                    let result = (|| -> anyhow::Result<()> {
                        let img = image::open(file_path)?;
                        let (current_w, current_h) = img.dimensions();

                        // Determine if we need to fit or fill
                        // For App Store screenshots, we want to fit within dimensions
                        let aspect_ratio = current_w as f32 / current_h as f32;
                        let target_aspect = target_width as f32 / target_height as f32;

                        let resized = if (aspect_ratio - target_aspect).abs() < 0.01 {
                            // Same aspect ratio - just resize
                            img.resize_exact(target_width, target_height, FilterType::Lanczos3)
                        } else {
                            // Different aspect ratio - fit within bounds
                            img.resize(target_width, target_height, FilterType::Lanczos3)
                        };

                        resized.save(&output_path)?;
                        Ok(())
                    })();

                    pb.inc(1);

                    let display_name = file_name.to_string_lossy();
                    match result {
                        Ok(_) => {
                            pb.set_message(format!("✓ {display_name}"));
                            Ok(file_path.clone())
                        }
                        Err(e) => {
                            pb.set_message(format!("✗ {display_name}"));
                            Err((file_path.clone(), e.to_string()))
                        }
                    }
                })
                .collect();

            pb.finish_with_message("Done");

            // Report results
            let successes: Vec<_> = results.iter().filter_map(|r| r.as_ref().ok()).collect();
            let failures: Vec<_> = results.iter().filter_map(|r| r.as_ref().err()).collect();

            println!("\n✓ Successfully processed {} file(s)", successes.len());
            println!("Output directory: {}", output.display());
            
            if !failures.is_empty() {
                println!("✗ Failed to process {} file(s):", failures.len());
                for (path, err) in failures {
                    println!("  {}: {}", path.display(), err);
                }
            }
        }
    }

    Ok(())
}

/// Parse hex color string to RGB array
fn parse_hex_color(hex: &str) -> anyhow::Result<[u8; 3]> {
    let hex = hex.trim_start_matches('#');
    
    if hex.len() != 6 {
        anyhow::bail!("Invalid hex color format. Expected 6 characters (e.g., ffffff)");
    }
    
    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;
    
    Ok([r, g, b])
}

/// Check if a file is an image based on extension
fn is_image_file(path: &std::path::Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tiff" | "tif")
    } else {
        false
    }
}
