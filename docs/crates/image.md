# foodshare-image

Image format detection and basic processing.

## Installation

```toml
[dependencies]
foodshare-image = "1.4"
```

## Features

- Format detection from bytes
- Image dimensions extraction
- Basic validation
- Supported formats: JPEG, PNG, WebP, GIF

## Usage

### Format Detection

```rust
use foodshare_image::{detect_format, ImageFormat};

let format = detect_format(&bytes)?;
match format {
    ImageFormat::Jpeg => println!("JPEG image"),
    ImageFormat::Png => println!("PNG image"),
    ImageFormat::WebP => println!("WebP image"),
    ImageFormat::Gif => println!("GIF image"),
    ImageFormat::Unknown => println!("Unknown format"),
}
```

### Metadata

```rust
use foodshare_image::extract_metadata;

let metadata = extract_metadata(&bytes)?;
println!("{}x{}", metadata.width, metadata.height);
```

### Resizing

```rust
use foodshare_image::{fit_within, resize_image, FitOptions, ResizeOptions};

// To a target width, with the height calculated
let resized = resize_image(&bytes, &ResizeOptions {
    width: 800,
    quality: 85,
    format: None,
})?;

// Inside a set of bounds, keeping the shape
let fitted = fit_within(&bytes, &FitOptions {
    max_width: 1242,
    max_height: 2688,
    quality: 90,
    format: None,
})?;
```

## CLI Usage

```bash
# Detect format
fs-image detect image.jpg

# Get the metadata
fs-image metadata image.jpg

# Validate
fs-image analyze image.jpg
```

## Magic Bytes

| Format | Magic Bytes |
|--------|-------------|
| JPEG | `FF D8 FF` |
| PNG | `89 50 4E 47` |
| WebP | `52 49 46 46` |
| GIF | `47 49 46 38` |

## Links

- [crates.io](https://crates.io/crates/foodshare-image)
