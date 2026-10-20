# foodshare-image

Image processing utilities for format detection, metadata extraction, and smart resizing.

[![Crates.io](https://img.shields.io/crates/v/foodshare-image.svg)](https://crates.io/crates/foodshare-image)
[![Documentation](https://docs.rs/foodshare-image/badge.svg)](https://docs.rs/foodshare-image)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **Format Detection** - Identify image format from magic bytes (not file extension)
- **Metadata Extraction** - Get dimensions without decoding entire image
- **Smart Width Calculation** - Optimal resize dimensions based on file size
- **Image Resizing** - Resize images with quality preservation (optional feature)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
foodshare-image = "1.3"

# With image processing (adds ~2MB to binary)
foodshare-image = { version = "1.3", features = ["processing"] }
```

### Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `processing` | No | Enable image resize/optimization |

## Usage

### Format Detection

Detect image format from magic bytes (first few bytes of file):

```rust
use foodshare_image::{detect_format, ImageFormat};

let jpeg_bytes = &[0xFF, 0xD8, 0xFF, 0xE0];
let format = detect_format(jpeg_bytes);
assert_eq!(format, Some(ImageFormat::Jpeg));

let png_bytes = &[0x89, 0x50, 0x4E, 0x47];
let format = detect_format(png_bytes);
assert_eq!(format, Some(ImageFormat::Png));
```

### Supported Formats

| Format | Magic Bytes | Extension |
|--------|-------------|-----------|
| JPEG | `FF D8 FF` | .jpg, .jpeg |
| PNG | `89 50 4E 47` | .png |
| GIF | `47 49 46 38` | .gif |
| WebP | `52 49 46 46...57 45 42 50` | .webp |
| AVIF | `...66 74 79 70 61 76 69 66` | .avif |
| HEIC | `...66 74 79 70 68 65 69 63` | .heic |
| BMP | `42 4D` | .bmp |
| TIFF | `49 49 2A 00` or `4D 4D 00 2A` | .tiff |
| ICO | `00 00 01 00` | .ico |
| SVG | `3C 73 76 67` or `3C 3F 78 6D 6C` | .svg |

### Metadata Extraction

Get image dimensions without fully decoding:

```rust
use foodshare_image::extract_metadata;

let metadata = extract_metadata(&image_bytes)?;
println!("Size: {}x{}", metadata.width, metadata.height);
println!("Format: {:?}", metadata.format);
```

### Smart Width Calculation

Calculate optimal resize width based on file size tiers:

```rust
use foodshare_image::{calculate_target_width, SizeTier};

let file_size = 2_500_000; // 2.5 MB
let original_width = 4000;

let target = calculate_target_width(file_size, original_width);
// Returns optimal width for the size tier

// Size tiers:
// - Tiny: < 100 KB -> no resize
// - Small: 100 KB - 500 KB -> max 1200px
// - Medium: 500 KB - 2 MB -> max 1600px
// - Large: 2 MB - 5 MB -> max 2000px
// - XLarge: > 5 MB -> max 2400px
```

### Image Resizing (requires `processing` feature)

```rust
use foodshare_image::{resize_image, ResizeOptions};

let options = ResizeOptions {
    max_width: 1200,
    max_height: 1200,
    quality: 85,
    preserve_aspect_ratio: true,
};

let resized = resize_image(&image_bytes, options)?;
```

## CLI Tool

The `fs-image` binary provides command-line access:

```bash
# Detect format
fs-image detect image.jpg

# Get metadata
fs-image metadata image.png

# Resize image
fs-image resize input.jpg -o output.jpg --width 1200 --quality 85
```

## Use Cases

### Pre-upload Optimization

```rust
use foodshare_image::{detect_format, calculate_target_width, ImageFormat};

fn should_resize(bytes: &[u8], file_size: usize) -> Option<u32> {
    let format = detect_format(bytes)?;
    
    // Only resize raster formats
    match format {
        ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP => {
            let metadata = extract_metadata(bytes).ok()?;
            let target = calculate_target_width(file_size, metadata.width);
            
            if target < metadata.width {
                Some(target)
            } else {
                None
            }
        }
        _ => None
    }
}
```

### Format Validation

```rust
use foodshare_image::{detect_format, ImageFormat};

fn validate_upload(bytes: &[u8]) -> Result<(), &'static str> {
    match detect_format(bytes) {
        Some(ImageFormat::Jpeg) | Some(ImageFormat::Png) | Some(ImageFormat::WebP) => Ok(()),
        Some(_) => Err("Unsupported image format"),
        None => Err("Not a valid image"),
    }
}
```

## Performance

- Format detection: ~10ns (just reads first few bytes)
- Metadata extraction: ~1µs (reads headers only)
- Full resize: depends on image size

## License

MIT License - see [LICENSE](LICENSE) for details.
