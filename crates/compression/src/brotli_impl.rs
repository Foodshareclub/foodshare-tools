//! Brotli compression implementation.

use crate::{CompressionError, Result};
use std::io::{Read, Write};

/// Compress data using Brotli.
///
/// # Arguments
/// * `data` - Data to compress
/// * `quality` - Compression quality (0-11, default 6)
///
/// # Returns
/// Compressed data
pub fn brotli_compress(data: &[u8], quality: u32) -> Result<Vec<u8>> {
    let quality = quality.min(11);
    let mut output = Vec::new();

    {
        let mut encoder = brotli::CompressorWriter::new(&mut output, 4096, quality, 22);
        encoder.write_all(data)?;
    }

    Ok(output)
}

/// Decompress Brotli data.
///
/// # Arguments
/// * `data` - Compressed data
///
/// # Returns
/// Decompressed data
pub fn brotli_decompress(data: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut decoder = brotli::Decompressor::new(data, 4096);
    decoder.read_to_end(&mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let original = b"Hello, World! This is a test of Brotli compression.";
        let compressed = brotli_compress(original, 6).unwrap();
        let decompressed = brotli_decompress(&compressed).unwrap();
        assert_eq!(original.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_compression_ratio() {
        let data = "a".repeat(1000);
        let compressed = brotli_compress(data.as_bytes(), 11).unwrap();
        assert!(compressed.len() < data.len() / 10);
    }
}
