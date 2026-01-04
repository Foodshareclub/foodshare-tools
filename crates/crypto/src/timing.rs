//! Constant-time operations for security.

use subtle::ConstantTimeEq;

/// Compare two byte slices in constant time.
///
/// This prevents timing attacks by ensuring the comparison
/// takes the same amount of time regardless of where differences occur.
///
/// # Arguments
/// * `a` - First byte slice
/// * `b` - Second byte slice
///
/// # Returns
/// true if slices are equal, false otherwise
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equal_slices() {
        assert!(constant_time_compare(b"hello", b"hello"));
    }

    #[test]
    fn test_different_slices() {
        assert!(!constant_time_compare(b"hello", b"world"));
    }

    #[test]
    fn test_different_lengths() {
        assert!(!constant_time_compare(b"hello", b"hi"));
    }

    #[test]
    fn test_empty_slices() {
        assert!(constant_time_compare(b"", b""));
    }
}
