//! Image format detection from magic bytes.

use crate::{ImageError, Result};

/// Supported image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    /// JPEG image
    Jpeg,
    /// PNG image
    Png,
    /// GIF image
    Gif,
    /// WebP image
    WebP,
    /// AVIF image
    Avif,
    /// BMP image
    Bmp,
    /// TIFF image
    Tiff,
    /// HEIC/HEIF image
    Heic,
}

impl ImageFormat {
    /// Get the MIME type for this format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png => "image/png",
            ImageFormat::Gif => "image/gif",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Avif => "image/avif",
            ImageFormat::Bmp => "image/bmp",
            ImageFormat::Tiff => "image/tiff",
            ImageFormat::Heic => "image/heic",
        }
    }

    /// Get common file extensions for this format.
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            ImageFormat::Jpeg => &["jpg", "jpeg"],
            ImageFormat::Png => &["png"],
            ImageFormat::Gif => &["gif"],
            ImageFormat::WebP => &["webp"],
            ImageFormat::Avif => &["avif"],
            ImageFormat::Bmp => &["bmp"],
            ImageFormat::Tiff => &["tiff", "tif"],
            ImageFormat::Heic => &["heic", "heif"],
        }
    }
}

/// Detect image format from magic bytes.
///
/// # Arguments
/// * `data` - First few bytes of the image file (at least 12 bytes recommended)
///
/// # Returns
/// Detected image format, or error if unknown
///
/// # Example
/// ```
/// use foodshare_image::detect_format;
///
/// // JPEG magic bytes
/// let jpeg_data = [0xFF, 0xD8, 0xFF, 0xE0];
/// assert!(matches!(detect_format(&jpeg_data), Ok(foodshare_image::ImageFormat::Jpeg)));
///
/// // PNG magic bytes
/// let png_data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
/// assert!(matches!(detect_format(&png_data), Ok(foodshare_image::ImageFormat::Png)));
/// ```
pub fn detect_format(data: &[u8]) -> Result<ImageFormat> {
    if data.len() < 4 {
        return Err(ImageError::InvalidData("Not enough data for format detection".into()));
    }

    // JPEG: FF D8 FF
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Ok(ImageFormat::Jpeg);
    }

    // PNG: 89 50 4E 47 0D 0A 1A 0A
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Ok(ImageFormat::Png);
    }

    // GIF: GIF87a or GIF89a
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return Ok(ImageFormat::Gif);
    }

    // WebP: RIFF....WEBP
    if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        return Ok(ImageFormat::WebP);
    }

    // BMP: BM
    if data.starts_with(b"BM") {
        return Ok(ImageFormat::Bmp);
    }

    // TIFF: II or MM (little/big endian)
    if data.starts_with(&[0x49, 0x49, 0x2A, 0x00]) || data.starts_with(&[0x4D, 0x4D, 0x00, 0x2A]) {
        return Ok(ImageFormat::Tiff);
    }

    // AVIF: ....ftypavif or ....ftypavis
    if data.len() >= 12 {
        if &data[4..8] == b"ftyp" {
            let brand = &data[8..12];
            if brand == b"avif" || brand == b"avis" || brand == b"mif1" {
                return Ok(ImageFormat::Avif);
            }
            // HEIC: ....ftypheic or ....ftypheix
            if brand == b"heic" || brand == b"heix" || brand == b"mif1" {
                return Ok(ImageFormat::Heic);
            }
        }
    }

    Err(ImageError::UnknownFormat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_jpeg() {
        let data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
        assert_eq!(detect_format(&data).unwrap(), ImageFormat::Jpeg);
    }

    #[test]
    fn test_detect_png() {
        let data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        assert_eq!(detect_format(&data).unwrap(), ImageFormat::Png);
    }

    #[test]
    fn test_detect_gif() {
        let data = b"GIF89a\x00\x00\x00\x00";
        assert_eq!(detect_format(data).unwrap(), ImageFormat::Gif);
    }

    #[test]
    fn test_detect_webp() {
        let data = b"RIFF\x00\x00\x00\x00WEBP";
        assert_eq!(detect_format(data).unwrap(), ImageFormat::WebP);
    }

    #[test]
    fn test_unknown_format() {
        let data = [0x00, 0x00, 0x00, 0x00];
        assert!(detect_format(&data).is_err());
    }

    #[test]
    fn test_mime_types() {
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert_eq!(ImageFormat::WebP.mime_type(), "image/webp");
    }
}
