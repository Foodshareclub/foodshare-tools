//! Image resizing with the image crate.

use crate::{ImageError, Result, ImageFormat, detect_format, smart_width::calculate_dimensions};
use image::{DynamicImage, ImageOutputFormat};
use std::io::Cursor;

/// Options for image resizing.
#[derive(Debug, Clone)]
pub struct ResizeOptions {
    /// Target width (height calculated automatically)
    pub width: u32,
    /// JPEG quality (1-100)
    pub quality: u8,
    /// Output format (None = same as input)
    pub format: Option<ImageFormat>,
}

impl Default for ResizeOptions {
    fn default() -> Self {
        Self {
            width: 800,
            quality: 85,
            format: None,
        }
    }
}

/// Resize an image.
///
/// # Arguments
/// * `data` - Image file data
/// * `options` - Resize options
///
/// # Returns
/// Resized image data
pub fn resize_image(data: &[u8], options: &ResizeOptions) -> Result<Vec<u8>> {
    let input_format = detect_format(data)?;
    let img = image::load_from_memory(data)?;

    let current_width = img.width();
    let current_height = img.height();

    // Calculate new dimensions
    let (new_width, new_height) = calculate_dimensions(current_width, current_height, options.width);

    // Resize if needed
    let resized = if new_width != current_width {
        img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    // Encode to output format
    let output_format = options.format.unwrap_or(input_format);
    encode_image(&resized, output_format, options.quality)
}

/// Encode a DynamicImage to bytes.
fn encode_image(img: &DynamicImage, format: ImageFormat, quality: u8) -> Result<Vec<u8>> {
    let mut buffer = Cursor::new(Vec::new());

    let output_format = match format {
        ImageFormat::Jpeg => ImageOutputFormat::Jpeg(quality),
        ImageFormat::Png => ImageOutputFormat::Png,
        ImageFormat::Gif => ImageOutputFormat::Gif,
        ImageFormat::WebP => ImageOutputFormat::WebP,
        _ => return Err(ImageError::ResizeError(format!("Unsupported output format: {:?}", format))),
    };

    img.write_to(&mut buffer, output_format)?;
    Ok(buffer.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would need actual image data to run
    // In practice, you'd use test fixtures

    #[test]
    fn test_resize_options_default() {
        let opts = ResizeOptions::default();
        assert_eq!(opts.width, 800);
        assert_eq!(opts.quality, 85);
        assert!(opts.format.is_none());
    }
}
