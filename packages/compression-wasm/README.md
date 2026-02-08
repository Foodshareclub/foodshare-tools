# @foodshare/compression-wasm

Response compression utilities with Brotli support compiled to WebAssembly from Rust.

[![npm version](https://img.shields.io/npm/v/@foodshare/compression-wasm.svg)](https://www.npmjs.com/package/@foodshare/compression-wasm)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **Brotli Compression** - Better compression ratios than Gzip (not available in Deno!)
- **Gzip/Deflate** - Standard compression formats
- **ETag Generation** - SHA-256 based ETags for caching
- **Auto Algorithm** - Automatic compression selection based on payload size
- **TypeScript Support** - Full type definitions included

## Why This Package?

Deno's `CompressionStream` doesn't support Brotli! This package brings Brotli compression to Supabase Edge Functions and other Deno runtimes via WebAssembly.

## Installation

```bash
bun add @foodshare/compression-wasm
```

## Usage

### Initialization

```typescript
import init, { brotli_compress, gzip_compress } from '@foodshare/compression-wasm';

// Initialize WASM module (required once)
await init();
```

### Brotli Compression

```typescript
import init, { brotli_compress, brotli_decompress } from '@foodshare/compression-wasm';

await init();

const data = new TextEncoder().encode('Hello World!'.repeat(100));

// Compress with quality 4 (0-11, higher = better compression)
const compressed = brotli_compress(data, 4);
console.log(`Compressed: ${data.length} -> ${compressed.length} bytes`);

// Decompress
const decompressed = brotli_decompress(compressed);
const text = new TextDecoder().decode(decompressed);
```

### Gzip Compression

```typescript
import init, { gzip_compress, gzip_decompress } from '@foodshare/compression-wasm';

await init();

const data = new TextEncoder().encode('Hello World!');

// Compress with level 6 (0-9)
const compressed = gzip_compress(data, 6);

// Decompress
const decompressed = gzip_decompress(compressed);
```

### Auto Compression

```typescript
import init, { compress_auto } from '@foodshare/compression-wasm';

await init();

// Automatically selects:
// - Brotli for payloads > 1KB
// - Gzip for smaller payloads
const compressed = compress_auto(data);
```

### ETag Generation

```typescript
import init, { generate_etag } from '@foodshare/compression-wasm';

await init();

const data = new TextEncoder().encode('response body');
const etag = generate_etag(data);
// Returns: "a1b2c3..." (SHA-256 hash)
```

## API Reference

### `brotli_compress(data, quality): Uint8Array`
Compress data using Brotli. Quality: 0-11 (higher = better compression, slower).

### `brotli_decompress(data): Uint8Array`
Decompress Brotli data.

### `gzip_compress(data, level): Uint8Array`
Compress data using Gzip. Level: 0-9.

### `gzip_decompress(data): Uint8Array`
Decompress Gzip data.

### `compress_auto(data): Uint8Array`
Auto-select compression based on size (Brotli > 1KB, else Gzip).

### `generate_etag(data): string`
Generate SHA-256 based ETag for caching.

## Supabase Edge Function Example

```typescript
import { serve } from 'https://deno.land/std@0.168.0/http/server.ts';
import init, { brotli_compress, generate_etag } from '@foodshare/compression-wasm';

await init();

serve(async (req) => {
  const data = JSON.stringify({ message: 'Hello from Edge!' });
  const body = new TextEncoder().encode(data);

  // Check if client accepts Brotli
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

  return new Response(body, {
    headers: { 'Content-Type': 'application/json' },
  });
});
```

## Compression Comparison

| Algorithm | Ratio | Speed | Browser Support |
|-----------|-------|-------|-----------------|
| Brotli | Best | Slower | All modern |
| Gzip | Good | Fast | Universal |
| Deflate | Good | Fast | Universal |

## Performance

- Brotli compression ~100KB in ~200µs
- Gzip compression ~100KB in ~100µs
- Zero-copy operations with WASM

## Browser Support

Works in all modern browsers with WebAssembly support:
- Chrome 57+
- Firefox 52+
- Safari 11+
- Edge 16+

## License

MIT License - see [LICENSE](https://github.com/Foodshareclub/foodshare-tools/blob/main/LICENSE) for details.

## Related

- [foodshare-compression](https://crates.io/crates/foodshare-compression) - The Rust crate this package is built from
