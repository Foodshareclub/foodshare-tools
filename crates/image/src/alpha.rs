//! Alpha channel removal utilities.

use crate::error::{ImageError, Result};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use std::path::Path;

/// Options for alpha channel removal
#[derive(Debug, Clone)]
pub struct AlphaRemovalOptions {
    /// Background color to use when removing alpha (RGB)
    pub background_color: [u8; 3],
    /// Whether to overwrite the original file
    pub overwrite: bool,
    /// Output format (if None, uses input format)
    pub output_format: Option<image::ImageFormat>,
}

impl Default for AlphaRemovalOptions {
    fn default() -> Self {
        Self {
            background_color: [255, 255, 255], // White background
            overwrite: false,
            output_format: None,
        }
    }
}

/// Remove alpha channel from an image by compositing over a solid background
pub fn remove_alpha_channel(
    img: &DynamicImage,
    background_color: [u8; 3],
) -> DynamicImage {
    let (width, height) = img.dimensions();
    let rgba_img = img.to_rgba8();
    
    let mut output = ImageBuffer::new(width, height);
    
    for (x, y, pixel) in rgba_img.enumerate_pixels() {
        let Rgba([r, g, b, a]) = *pixel;
        
        // Alpha blending: composite over background
        let alpha = a as f32 / 255.0;
        let inv_alpha = 1.0 - alpha;
        
        let new_r = ((r as f32 * alpha) + (background_color[0] as f32 * inv_alpha)) as u8;
        let new_g = ((g as f32 * alpha) + (background_color[1] as f32 * inv_alpha)) as u8;
        let new_b = ((b as f32 * alpha) + (background_color[2] as f32 * inv_alpha)) as u8;
        
        output.put_pixel(x, y, Rgba([new_r, new_g, new_b, 255]));
    }
    
    DynamicImage::ImageRgba8(output)
}

/// Process a single image file to remove alpha channel
pub fn process_image_file(
    input_path: &Path,
    output_path: &Path,
    options: &AlphaRemovalOptions,
) -> Result<()> {
    // Load the image
    let img = image::open(input_path)?;
    
    // Check if image has alpha channel
    if !has_alpha_channel(&img) {
        return Err(ImageError::InvalidData(
            "Image does not have an alpha channel".to_string(),
        ));
    }
    
    // Remove alpha channel
    let processed = remove_alpha_channel(&img, options.background_color);
    
    // Determine output format
    let format = options.output_format.or_else(|| {
        image::ImageFormat::from_path(input_path).ok()
    });
    
    // Save the image
    if let Some(fmt) = format {
        processed.save_with_format(output_path, fmt)?;
    } else {
        processed.save(output_path)?;
    }
    
    Ok(())
}

/// Check if an image has an alpha channel
pub fn has_alpha_channel(img: &DynamicImage) -> bool {
    matches!(
        img,
        DynamicImage::ImageRgba8(_)
            | DynamicImage::ImageRgba16(_)
            | DynamicImage::ImageRgba32F(_)
            | DynamicImage::ImageLumaA8(_)
            | DynamicImage::ImageLumaA16(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn test_remove_alpha_white_background() {
        let mut img = RgbaImage::new(2, 2);
        img.put_pixel(0, 0, Rgba([255, 0, 0, 255])); // Opaque red
        img.put_pixel(0, 1, Rgba([0, 255, 0, 128])); // Semi-transparent green
        img.put_pixel(1, 0, Rgba([0, 0, 255, 0]));   // Fully transparent blue
        img.put_pixel(1, 1, Rgba([255, 255, 0, 255])); // Opaque yellow
        
        let dynamic = DynamicImage::ImageRgba8(img);
        let result = remove_alpha_channel(&dynamic, [255, 255, 255]);
        
        let result_rgba = result.to_rgba8();
        
        // Check opaque red stays the same
        assert_eq!(result_rgba.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
        
        // Check semi-transparent green blends with white
        let pixel = result_rgba.get_pixel(0, 1);
        assert_eq!(pixel[3], 255); // Alpha should be 255
        
        // Check fully transparent blue becomes white
        assert_eq!(result_rgba.get_pixel(1, 0), &Rgba([255, 255, 255, 255]));
        
        // Check opaque yellow stays the same
        assert_eq!(result_rgba.get_pixel(1, 1), &Rgba([255, 255, 0, 255]));
    }

    #[test]
    fn test_has_alpha_channel() {
        let rgba_img = DynamicImage::ImageRgba8(RgbaImage::new(1, 1));
        assert!(has_alpha_channel(&rgba_img));
        
        let rgb_img = DynamicImage::ImageRgb8(image::RgbImage::new(1, 1));
        assert!(!has_alpha_channel(&rgb_img));
    }
}
