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

### Dimensions

```rust
use foodshare_image::dimensions;

let (width, height) = dimensions(&bytes)?;
println!("{}x{}", width, height);
```

### Validation

```rust
use foodshare_image::validate;

// Validate image
let result = validate(&bytes, &ValidationConfig {
    max_width: 4096,
    max_height: 4096,
    max_size: 5_000_000, // 5MB
    allowed_formats: vec![ImageFormat::Jpeg, ImageFormat::Png],
})?;
```

## CLI Usage

```bash
# Detect format
fs-image detect image.jpg

# Get dimensions
fs-image dimensions image.jpg

# Validate
fs-image validate image.jpg --max-size 5MB
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
