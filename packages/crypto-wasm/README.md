# @foodshare/crypto-wasm

Cryptographic utilities for webhook verification compiled to WebAssembly from Rust.

[![npm version](https://img.shields.io/npm/v/@foodshare/crypto-wasm.svg)](https://www.npmjs.com/package/@foodshare/crypto-wasm)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **HMAC-SHA256/SHA1** - Generate HMAC signatures for webhook verification
- **Constant-Time Comparison** - Secure signature verification resistant to timing attacks
- **Multiple Output Formats** - Hex and Base64 encoding support
- **TypeScript Support** - Full type definitions included

## Installation

```bash
bun add @foodshare/crypto-wasm
```

## Usage

### Initialization

```typescript
import init, { hmac_sha256_hex, verify_webhook_sha256 } from '@foodshare/crypto-wasm';

// Initialize WASM module (required once)
await init();
```

### Generate HMAC Signature

```typescript
import init, { hmac_sha256_hex, hmac_sha1_hex } from '@foodshare/crypto-wasm';

await init();

// HMAC-SHA256 (most providers)
const signature = hmac_sha256_hex('your-secret-key', 'payload-to-sign');

// HMAC-SHA1 (GitHub webhooks)
const sha1Sig = hmac_sha1_hex('your-secret-key', 'payload-to-sign');
```

### Verify Webhook Signatures

```typescript
import init, { verify_webhook_sha256 } from '@foodshare/crypto-wasm';

await init();

function verifyStripeWebhook(payload: string, signature: string, secret: string): boolean {
  return verify_webhook_sha256(secret, payload, signature);
}

// Example: Stripe webhook verification
const isValid = verifyStripeWebhook(
  req.body,
  req.headers['stripe-signature'],
  process.env.STRIPE_WEBHOOK_SECRET
);
```

### Base64 Output

```typescript
import init, { hmac_sha256_base64 } from '@foodshare/crypto-wasm';

await init();

// Some providers expect Base64 signatures
const signature = hmac_sha256_base64('secret', 'message');
```

### Constant-Time Comparison

```typescript
import init, { constant_time_eq } from '@foodshare/crypto-wasm';

await init();

// Safe comparison resistant to timing attacks
const isEqual = constant_time_eq('signature1', 'signature2');
```

## API Reference

### `hmac_sha256_hex(key, message): string`
Generate HMAC-SHA256 signature as hex string.

### `hmac_sha256_base64(key, message): string`
Generate HMAC-SHA256 signature as base64 string.

### `hmac_sha1_hex(key, message): string`
Generate HMAC-SHA1 signature as hex string (for legacy providers).

### `verify_webhook_sha256(key, message, signature_hex): boolean`
Verify a webhook signature using constant-time comparison.

### `verify_webhook_sha1(key, message, signature_hex): boolean`
Verify SHA1 webhook signature (GitHub).

### `constant_time_eq(a, b): boolean`
Constant-time string comparison.

## Provider Examples

### Stripe

```typescript
const isValid = verify_webhook_sha256(
  process.env.STRIPE_SECRET,
  payload,
  signatureHeader
);
```

### GitHub

```typescript
// GitHub uses SHA1 with 'sha1=' prefix
const signature = req.headers['x-hub-signature'].replace('sha1=', '');
const isValid = verify_webhook_sha1(
  process.env.GITHUB_SECRET,
  payload,
  signature
);
```

### Meta/Facebook

```typescript
const signature = req.headers['x-hub-signature-256'].replace('sha256=', '');
const isValid = verify_webhook_sha256(
  process.env.META_SECRET,
  payload,
  signature
);
```

## Security

- All signature verification uses constant-time comparison to prevent timing attacks
- Built with Rust's `subtle` crate for cryptographic operations
- No external runtime dependencies in the WASM binary

## Browser Support

Works in all modern browsers with WebAssembly support:
- Chrome 57+
- Firefox 52+
- Safari 11+
- Edge 16+

## License

MIT License - see [LICENSE](https://github.com/Foodshareclub/foodshare-tools/blob/main/LICENSE) for details.

## Related

- [foodshare-crypto](https://crates.io/crates/foodshare-crypto) - The Rust crate this package is built from
