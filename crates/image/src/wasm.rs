//! WebAssembly bindings for FoodShare image processing & geometry utilities.

use wasm_bindgen::prelude::*;

/// Detect image format from magic bytes (JPEG, PNG, GIF, WebP, AVIF, BMP, TIFF, HEIC).
///
/// # Arguments
/// * `data` - Raw image byte buffer (at least first 12 bytes recommended)
///
/// # Returns
/// Format name as lowercase string (e.g. "jpeg", "png", "webp"), or None if unrecognized.
#[wasm_bindgen]
pub fn detect_image_format(data: &[u8]) -> Option<String> {
    crate::detect_format(data)
        .ok()
        .map(|f| format!("{:?}", f).to_lowercase())
}

/// Get the standard MIME type for an image byte buffer.
///
/// # Arguments
/// * `data` - Raw image byte buffer
///
/// # Returns
/// MIME type string (e.g. "image/jpeg", "image/png", "image/webp"), or None.
#[wasm_bindgen]
pub fn get_image_mime_type(data: &[u8]) -> Option<String> {
    crate::detect_format(data).ok().map(|f| f.mime_type().to_string())
}

/// Calculate optimal resized target width based on raw file size tiers and current dimensions.
///
/// Returns target width in pixels (0 means no resize needed).
#[wasm_bindgen]
pub fn calculate_smart_width(file_size_bytes: f64, current_width: u32, current_height: u32) -> u32 {
    let size = if file_size_bytes < 0.0 {
        0
    } else {
        file_size_bytes as usize
    };
    crate::smart_width::calculate_target_width(size, current_width, current_height)
}

/// Check if a byte buffer contains a valid recognized image format.
#[wasm_bindgen]
pub fn is_valid_image(data: &[u8]) -> bool {
    crate::detect_format(data).is_ok()
}

/// Extract image metadata (dimensions, format, aspect ratio, orientation) as a JSON string.
///
/// Supports instant zero-allocation parsing for JPEG, PNG, and GIF.
///
/// # Arguments
/// * `data` - Image byte buffer
///
/// # Returns
/// JSON string with metadata object, or None if extraction failed.
#[wasm_bindgen]
pub fn extract_image_metadata_json(data: &[u8]) -> Option<String> {
    let meta = crate::extract_metadata(data)?;
    serde_json::to_string(&meta).ok()
}
