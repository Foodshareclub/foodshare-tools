# foodshare-compression

Brotli and Gzip compression utilities.

## Installation

```toml
[dependencies]
foodshare-compression = "1.4"
```

## Features

- Brotli compression/decompression
- Gzip compression/decompression
- Streaming support
- WASM support

## Usage

### Brotli

```rust
use foodshare_compression::brotli;

// Compress
let compressed = brotli::compress(data, 11)?; // quality 0-11

// Decompress
let decompressed = brotli::decompress(&compressed)?;
```

### Gzip

```rust
use foodshare_compression::gzip;

// Compress
let compressed = gzip::compress(data, 9)?; // level 0-9

// Decompress
let decompressed = gzip::decompress(&compressed)?;
```

### Auto-detect

```rust
use foodshare_compression::decompress;

// Automatically detects format
let data = decompress(&compressed)?;
```

## WASM Usage

```typescript
import init, { brotli_compress, brotli_decompress } from '@foodshare/compression-wasm';

await init();

const compressed = brotli_compress(data, 11);
const decompressed = brotli_decompress(compressed);
```

## Compression Ratios

| Format | Ratio | Speed |
|--------|-------|-------|
| Brotli (11) | Best | Slow |
| Brotli (5) | Good | Medium |
| Gzip (9) | Good | Fast |

## Links

- [crates.io](https://crates.io/crates/foodshare-compression)
- [npm](https://www.npmjs.com/package/@foodshare/compression-wasm)
