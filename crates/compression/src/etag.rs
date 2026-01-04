//! ETag generation for HTTP caching.

use sha2::{Sha256, Digest};

/// Generate an ETag for content.
///
/// Uses SHA-256 hash of the content, truncated to 32 characters.
///
/// # Arguments
/// * `content` - Content to hash
///
/// # Returns
/// ETag string (quoted for HTTP header use)
pub fn generate_etag(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let hash = hasher.finalize();
    let hex = hex::encode(&hash[..16]); // Use first 16 bytes (32 hex chars)
    format!("\"{}\"", hex)
}

/// Generate a weak ETag for content.
///
/// Weak ETags indicate that the response is semantically equivalent
/// but not byte-for-byte identical.
///
/// # Arguments
/// * `content` - Content to hash
///
/// # Returns
/// Weak ETag string
pub fn generate_weak_etag(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let hash = hasher.finalize();
    let hex = hex::encode(&hash[..16]);
    format!("W/\"{}\"", hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_etag_format() {
        let etag = generate_etag(b"hello");
        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));
        assert_eq!(etag.len(), 34); // 32 hex + 2 quotes
    }

    #[test]
    fn test_weak_etag_format() {
        let etag = generate_weak_etag(b"hello");
        assert!(etag.starts_with("W/\""));
        assert!(etag.ends_with('"'));
    }

    #[test]
    fn test_same_content_same_etag() {
        let etag1 = generate_etag(b"hello");
        let etag2 = generate_etag(b"hello");
        assert_eq!(etag1, etag2);
    }

    #[test]
    fn test_different_content_different_etag() {
        let etag1 = generate_etag(b"hello");
        let etag2 = generate_etag(b"world");
        assert_ne!(etag1, etag2);
    }
}
