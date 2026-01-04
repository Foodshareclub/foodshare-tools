//! Smart width calculation based on file size tiers.
//!
//! This module determines optimal target widths for images based on their
//! original file size, following the patterns from the TypeScript implementation.

use serde::{Deserialize, Serialize};

/// File size tiers for determining target width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizeTier {
    /// Under 200KB - minimal or no resize needed
    Tiny,
    /// 200KB - 500KB
    Small,
    /// 500KB - 1MB
    Medium,
    /// 1MB - 2MB
    Large,
    /// 2MB - 5MB
    ExtraLarge,
    /// Over 5MB - aggressive resize
    Huge,
}

impl SizeTier {
    /// Determine size tier from file size in bytes.
    pub fn from_bytes(size: usize) -> Self {
        const KB: usize = 1024;
        const MB: usize = 1024 * KB;

        match size {
            s if s < 200 * KB => SizeTier::Tiny,
            s if s < 500 * KB => SizeTier::Small,
            s if s < MB => SizeTier::Medium,
            s if s < 2 * MB => SizeTier::Large,
            s if s < 5 * MB => SizeTier::ExtraLarge,
            _ => SizeTier::Huge,
        }
    }

    /// Get recommended target width for this tier.
    pub fn target_width(&self) -> u32 {
        match self {
            SizeTier::Tiny => 0, // No resize needed
            SizeTier::Small => 1200,
            SizeTier::Medium => 1000,
            SizeTier::Large => 800,
            SizeTier::ExtraLarge => 600,
            SizeTier::Huge => 500,
        }
    }
}

/// Calculate target width for an image based on its file size and dimensions.
///
/// # Arguments
/// * `size_bytes` - File size in bytes
/// * `current_width` - Current image width
/// * `current_height` - Current image height
///
/// # Returns
/// Target width (0 means no resize needed)
///
/// # Example
/// ```
/// use foodshare_image::calculate_target_width;
///
/// // 3MB image at 4000px wide
/// let target = calculate_target_width(3 * 1024 * 1024, 4000, 3000);
/// assert_eq!(target, 600); // ExtraLarge tier
///
/// // 100KB image - no resize needed
/// let target = calculate_target_width(100 * 1024, 800, 600);
/// assert_eq!(target, 0);
/// ```
pub fn calculate_target_width(size_bytes: usize, current_width: u32, current_height: u32) -> u32 {
    let tier = SizeTier::from_bytes(size_bytes);
    let target = tier.target_width();

    // Don't upscale
    if target == 0 || current_width <= target {
        return 0;
    }

    // Don't resize if image is already small enough
    let is_small = current_width <= 1200 && current_height <= 1200;
    if is_small && size_bytes < 500 * 1024 {
        return 0;
    }

    target
}

/// Calculate target dimensions maintaining aspect ratio.
///
/// # Arguments
/// * `current_width` - Current width
/// * `current_height` - Current height
/// * `target_width` - Target width
///
/// # Returns
/// (new_width, new_height) maintaining aspect ratio
pub fn calculate_dimensions(current_width: u32, current_height: u32, target_width: u32) -> (u32, u32) {
    if target_width == 0 || current_width <= target_width {
        return (current_width, current_height);
    }

    let ratio = target_width as f64 / current_width as f64;
    let new_height = (current_height as f64 * ratio).round() as u32;

    (target_width, new_height.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KB: usize = 1024;
    const MB: usize = 1024 * KB;

    #[test]
    fn test_size_tier_detection() {
        assert_eq!(SizeTier::from_bytes(100 * KB), SizeTier::Tiny);
        assert_eq!(SizeTier::from_bytes(300 * KB), SizeTier::Small);
        assert_eq!(SizeTier::from_bytes(700 * KB), SizeTier::Medium);
        assert_eq!(SizeTier::from_bytes(MB + 500 * KB), SizeTier::Large);
        assert_eq!(SizeTier::from_bytes(3 * MB), SizeTier::ExtraLarge);
        assert_eq!(SizeTier::from_bytes(10 * MB), SizeTier::Huge);
    }

    #[test]
    fn test_target_width_small_image() {
        // Small image under 200KB - no resize
        assert_eq!(calculate_target_width(100 * KB, 800, 600), 0);
    }

    #[test]
    fn test_target_width_large_image() {
        // 3MB image - should get ExtraLarge tier target
        assert_eq!(calculate_target_width(3 * MB, 4000, 3000), 600);
    }

    #[test]
    fn test_no_upscale() {
        // Even for large file, don't upscale small image
        assert_eq!(calculate_target_width(3 * MB, 400, 300), 0);
    }

    #[test]
    fn test_dimension_calculation() {
        // 4000x3000 -> 800 wide
        let (w, h) = calculate_dimensions(4000, 3000, 800);
        assert_eq!(w, 800);
        assert_eq!(h, 600);
    }

    #[test]
    fn test_dimension_no_resize() {
        let (w, h) = calculate_dimensions(500, 400, 800);
        assert_eq!(w, 500);
        assert_eq!(h, 400);
    }
}
