//! Gzip and Deflate compression implementations.

use crate::{CompressionError, Result};
use flate2::read::{GzDecoder, DeflateDecoder};
use flate2::write::{GzEncoder, DeflateEncoder};
use flate2::Compression;
use std::io::{Read, Write};

/// Compress data using Gzip.
///
/// # Arguments
/// * `data` - Data to compress
/// * `level` - Compression level (0-9)
///
/// # Returns
/// Compressed data
pub fn gzip_compress(data: &[u8], level: u32) -> Result<Vec<u8>> {
    let level = Compression::new(level.min(9));
    let mut encoder = GzEncoder::new(Vec::new(), level);
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

/// Decompress Gzip data.
///
/// # Arguments
/// * `data` - Compressed data
///
/// # Returns
/// Decompressed data
pub fn gzip_decompress(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(data);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}

/// Compress data using Deflate.
///
/// # Arguments
/// * `data` - Data to compress
/// * `level` - Compression level (0-9)
///
/// # Returns
/// Compressed data
pub fn deflate_compress(data: &[u8], level: u32) -> Result<Vec<u8>> {
    let level = Compression::new(level.min(9));
    let mut encoder = DeflateEncoder::new(Vec::new(), level);
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

/// Decompress Deflate data.
///
/// # Arguments
/// * `data` - Compressed data
///
/// # Returns
/// Decompressed data
pub fn deflate_decompress(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = DeflateDecoder::new(data);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gzip_roundtrip() {
        let original = b"Hello, Gzip!";
        let compressed = gzip_compress(original, 6).unwrap();
        let decompressed = gzip_decompress(&compressed).unwrap();
        assert_eq!(original.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_deflate_roundtrip() {
        let original = b"Hello, Deflate!";
        let compressed = deflate_compress(original, 6).unwrap();
        let decompressed = deflate_decompress(&compressed).unwrap();
        assert_eq!(original.as_slice(), decompressed.as_slice());
    }
}
