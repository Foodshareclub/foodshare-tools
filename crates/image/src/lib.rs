//! Image processing utilities for FoodShare.
//!
//! This crate provides:
//! - Format detection from magic bytes
//! - Image resizing and optimization
//! - Metadata extraction
//! - Smart width calculation for file size tiers
//! - Alpha channel removal

#![warn(missing_docs)]

mod detect;
mod error;
mod metadata;
pub mod smart_width;

#[cfg(feature = "processing")]
mod resize;

#[cfg(feature = "processing")]
mod alpha;

pub use detect::{ImageFormat, detect_format};
pub use error::{ImageError, Result};
pub use metadata::{ImageMetadata, extract_metadata};
pub use smart_width::{SizeTier, calculate_target_width};

#[cfg(feature = "processing")]
pub use resize::{ResizeOptions, resize_image};

#[cfg(feature = "processing")]
pub use alpha::{AlphaRemovalOptions, has_alpha_channel, process_image_file, remove_alpha_channel};

#[cfg(feature = "wasm")]
mod wasm;

#[cfg(feature = "wasm")]
pub use wasm::*;
