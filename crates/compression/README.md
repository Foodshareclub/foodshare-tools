# foodshare-compression

Response compression utilities with Brotli, Gzip, and ETag support.

[![Crates.io](https://img.shields.io/crates/v/foodshare-compression.svg)](https://crates.io/crates/foodshare-compression)
[![Documentation](https://docs.rs/foodshare-compression/badge.svg)](https://docs.rs/foodshare-compression)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **Brotli Compression** - Best compression ratios (not available in Deno!)
- **Gzip/Deflate** - Standard compression formats
- **ETag Generation** - SHA-256 based cache validation
- **WASM Support** - Compile to WebAssembly for Deno/browser usage

## Why This Crate?

Deno's `CompressionStream` doesn't support Brotli! This crate brings Brotli compression to Supabase Edge Functions and other Deno runtimes via WebAssembly.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
foodshare-compression = "1.3"
```

### Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `wasm` | No | Enable WebAssembly bindings |

## Usage

### Brotli Compression

```rust
use foodshare_compression::{brotli_compress, brotli_decompress};

let data = b"Hello, World!".repeat(100);

// Compress with quality 4 (0-11, higher = better compression)
let compressed = brotli_compress(&data, 4)?;
println!("Compressed: {} -> {} bytes", data.len(), compressed.len());

// Decompress
let decompressed = brotli_decompress(&compressed)?;
assert_eq!(data, decompressed.as_slice());
```

### Gzip Compression

```rust
use foodshare_compression::{gzip_compress, gzip_decompress};

let data = b"Hello, World!";

// Compress with level 6 (0-9)
let compressed = gzip_compress(data, 6)?;

// Decompress
let decompressed = gzip_decompress(&compressed)?;
```

### Deflate Compression

```rust
use foodshare_compression::{deflate_compress, deflate_decompress};

let compressed = deflate_compress(b"data", 6)?;
let decompressed = deflate_decompress(&compressed)?;
```

### Auto Algorithm Selection

```rust
use foodshare_compression::{compress, decompress, Algorithm};

// Choose based on Accept-Encoding header
let algorithm = if accept_encoding.contains("br") {
    Algorithm::Brotli
} else if accept_encoding.contains("gzip") {
    Algorithm::Gzip
} else {
    Algorithm::Deflate
};

let compressed = compress(data, algorithm, 4)?;
```

### ETag Generation

Generate SHA-256 based ETags for HTTP caching:

```rust
use foodshare_compression::generate_etag;

let etag = generate_etag(b"response body");
// Returns: "a1b2c3d4..." (64 char hex string)
```

## Compression Comparison

| Algorithm | Ratio | Speed | Browser Support |
|-----------|-------|-------|-----------------|
| Brotli | Best | Slower | All modern |
| Gzip | Good | Fast | Universal |
| Deflate | Good | Fast | Universal |

## Supabase Edge Function Example

```typescript
import { serve } from 'https://deno.land/std/http/server.ts';
import init, { brotli_compress, generate_etag } from '@foodshare/compression-wasm';

await init();

serve(async (req) => {
  const data = JSON.stringify({ message: 'Hello!' });
  const body = new TextEncoder().encode(data);
  
  const acceptEncoding = req.headers.get('accept-encoding') || '';
  
  if (acceptEncoding.includes('br')) {
    const compressed = brotli_compress(body, 4);
    return new Response(compressed, {
      headers: {
        'Content-Type': 'application/json',
        'Content-Encoding': 'br',
        'ETag': generate_etag(body),
      },
    });
  }
  
  return new Response(body);
});
```

## WASM Usage

For Deno/browser usage, see [@foodshare/compression-wasm](https://www.npmjs.com/package/@foodshare/compression-wasm).

```typescript
import init, { brotli_compress, gzip_compress } from '@foodshare/compression-wasm';

await init();

const data = new TextEncoder().encode('Hello!');
const compressed = brotli_compress(data, 4);
```

## Performance

- Brotli ~100KB in ~200µs
- Gzip ~100KB in ~100µs
- Minimal memory overhead

## License

MIT License - see [LICENSE](LICENSE) for details.
