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
mod metadata;
pub mod smart_width;
mod error;

#[cfg(feature = "processing")]
mod resize;

#[cfg(feature = "processing")]
mod alpha;

pub use detect::{detect_format, ImageFormat};
pub use metadata::{ImageMetadata, extract_metadata};
pub use smart_width::{calculate_target_width, SizeTier};
pub use error::{ImageError, Result};

#[cfg(feature = "processing")]
pub use resize::{resize_image, ResizeOptions};

#[cfg(feature = "processing")]
pub use alpha::{remove_alpha_channel, process_image_file, has_alpha_channel, AlphaRemovalOptions};
