//! Image metadata extraction.

use crate::{ImageFormat, detect_format};
use serde::{Deserialize, Serialize};

/// Image metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Detected format
    pub format: ImageFormat,
    /// File size in bytes
    pub size_bytes: usize,
}

impl ImageMetadata {
    /// Calculate aspect ratio (width / height).
    pub fn aspect_ratio(&self) -> f64 {
        self.width as f64 / self.height as f64
    }

    /// Check if image is landscape orientation.
    pub fn is_landscape(&self) -> bool {
        self.width > self.height
    }

    /// Check if image is portrait orientation.
    pub fn is_portrait(&self) -> bool {
        self.height > self.width
    }

    /// Check if image is square.
    pub fn is_square(&self) -> bool {
        self.width == self.height
    }
}

/// Extract metadata from image data.
///
/// # Arguments
/// * `data` - Image file data
///
/// # Returns
/// Image metadata if extraction succeeds
pub fn extract_metadata(data: &[u8]) -> Option<ImageMetadata> {
    let format = detect_format(data).ok()?;

    let (width, height) = match format {
        ImageFormat::Jpeg => extract_jpeg_dimensions(data)?,
        ImageFormat::Png => extract_png_dimensions(data)?,
        ImageFormat::Gif => extract_gif_dimensions(data)?,
        _ => return None, // Other formats need the image crate
    };

    Some(ImageMetadata {
        width,
        height,
        format,
        size_bytes: data.len(),
    })
}

/// Extract dimensions from JPEG data.
fn extract_jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    // Skip SOI marker
    let mut i = 2;

    while i + 4 < data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }

        let marker = data[i + 1];

        // SOF markers contain dimensions
        if matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            if i + 9 < data.len() {
                let height = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32;
                let width = u16::from_be_bytes([data[i + 7], data[i + 8]]) as u32;
                return Some((width, height));
            }
        }

        // Skip to next marker
        if marker == 0xD8 || marker == 0xD9 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
        } else if i + 3 < data.len() {
            let length = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
            i += 2 + length;
        } else {
            break;
        }
    }

    None
}

/// Extract dimensions from PNG data.
fn extract_png_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    // PNG header is 8 bytes, IHDR chunk starts at byte 8
    if data.len() < 24 {
        return None;
    }

    // IHDR chunk: 4 bytes length + 4 bytes "IHDR" + 4 bytes width + 4 bytes height
    let chunk_type = &data[12..16];
    if chunk_type != b"IHDR" {
        return None;
    }

    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);

    Some((width, height))
}

/// Extract dimensions from GIF data.
fn extract_gif_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    // GIF header: 6 bytes signature + 2 bytes width + 2 bytes height
    if data.len() < 10 {
        return None;
    }

    let width = u16::from_le_bytes([data[6], data[7]]) as u32;
    let height = u16::from_le_bytes([data[8], data[9]]) as u32;

    Some((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aspect_ratio() {
        let meta = ImageMetadata {
            width: 1920,
            height: 1080,
            format: ImageFormat::Jpeg,
            size_bytes: 1000,
        };
        assert!((meta.aspect_ratio() - 16.0 / 9.0).abs() < 0.01);
    }

    #[test]
    fn test_orientation() {
        let landscape = ImageMetadata { width: 1920, height: 1080, format: ImageFormat::Jpeg, size_bytes: 0 };
        let portrait = ImageMetadata { width: 1080, height: 1920, format: ImageFormat::Jpeg, size_bytes: 0 };
        let square = ImageMetadata { width: 1000, height: 1000, format: ImageFormat::Jpeg, size_bytes: 0 };

        assert!(landscape.is_landscape());
        assert!(!landscape.is_portrait());

        assert!(portrait.is_portrait());
        assert!(!portrait.is_landscape());

        assert!(square.is_square());
    }
}
