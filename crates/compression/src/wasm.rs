//! WASM bindings for compression utilities.

use wasm_bindgen::prelude::*;

/// Compress data using Brotli.
///
/// # Arguments
/// * `data` - Data to compress
/// * `quality` - Compression level (0-11, higher = better compression but slower)
///
/// # Returns
/// Compressed data as Uint8Array
#[wasm_bindgen]
pub fn brotli_compress(data: &[u8], quality: u32) -> Vec<u8> {
    crate::brotli_compress(data, quality).unwrap_or_else(|_| Vec::new())
}

/// Decompress Brotli data.
#[wasm_bindgen]
pub fn brotli_decompress(data: &[u8]) -> Vec<u8> {
    crate::brotli_decompress(data).unwrap_or_else(|_| Vec::new())
}

/// Compress data using Gzip.
///
/// # Arguments
/// * `data` - Data to compress
/// * `level` - Compression level (0-9)
#[wasm_bindgen]
pub fn gzip_compress(data: &[u8], level: u32) -> Vec<u8> {
    crate::gzip_compress(data, level).unwrap_or_else(|_| Vec::new())
}

/// Decompress Gzip data.
#[wasm_bindgen]
pub fn gzip_decompress(data: &[u8]) -> Vec<u8> {
    crate::gzip_decompress(data).unwrap_or_else(|_| Vec::new())
}

/// Generate ETag for data (SHA-256 based).
#[wasm_bindgen]
pub fn generate_etag(data: &[u8]) -> String {
    crate::generate_etag(data)
}

/// Compress data with automatic algorithm selection based on size.
///
/// Uses Brotli for larger payloads (>1KB), Gzip otherwise.
#[wasm_bindgen]
pub fn compress_auto(data: &[u8]) -> Vec<u8> {
    if data.len() > 1024 {
        // Use Brotli for larger payloads
        crate::brotli_compress(data, 4).unwrap_or_else(|_| Vec::new())
    } else {
        // Use Gzip for smaller payloads
        crate::gzip_compress(data, 6).unwrap_or_else(|_| Vec::new())
    }
}
