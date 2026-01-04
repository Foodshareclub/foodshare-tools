//! Response compression utilities for FoodShare.
//!
//! This crate provides:
//! - Brotli compression (not available in Deno!)
//! - Gzip/Deflate compression
//! - ETag generation

mod brotli_impl;
mod gzip;
mod etag;
mod error;

#[cfg(feature = "wasm")]
mod wasm;

pub use brotli_impl::{brotli_compress, brotli_decompress};
pub use gzip::{gzip_compress, gzip_decompress, deflate_compress, deflate_decompress};
pub use etag::generate_etag;
pub use error::{CompressionError, Result};

/// Compression algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    /// Brotli compression (best ratio)
    Brotli,
    /// Gzip compression (widely supported)
    Gzip,
    /// Deflate compression
    Deflate,
}

/// Compress data using the specified algorithm.
pub fn compress(data: &[u8], algorithm: Algorithm, level: u32) -> Result<Vec<u8>> {
    match algorithm {
        Algorithm::Brotli => brotli_compress(data, level),
        Algorithm::Gzip => gzip_compress(data, level),
        Algorithm::Deflate => deflate_compress(data, level),
    }
}

/// Decompress data using the specified algorithm.
pub fn decompress(data: &[u8], algorithm: Algorithm) -> Result<Vec<u8>> {
    match algorithm {
        Algorithm::Brotli => brotli_decompress(data),
        Algorithm::Gzip => gzip_decompress(data),
        Algorithm::Deflate => deflate_decompress(data),
    }
}
