# foodshare-crypto

Cryptographic utilities for webhook verification and HMAC signature generation.

[![Crates.io](https://img.shields.io/crates/v/foodshare-crypto.svg)](https://crates.io/crates/foodshare-crypto)
[![Documentation](https://docs.rs/foodshare-crypto/badge.svg)](https://docs.rs/foodshare-crypto)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **HMAC-SHA256/SHA1** - Generate signatures for webhook verification
- **Constant-Time Comparison** - Secure signature verification resistant to timing attacks
- **Provider Support** - Works with Stripe, GitHub, Meta/Facebook webhooks
- **WASM Support** - Compile to WebAssembly for browser/Deno usage

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
foodshare-crypto = "1.3"
```

### Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `wasm` | No | Enable WebAssembly bindings |

## Usage

### Generate HMAC Signature

```rust
use foodshare_crypto::{hmac_sha256, hmac_sha1};

// HMAC-SHA256 (Stripe, Meta)
let signature = hmac_sha256(b"your-secret-key", b"payload");

// HMAC-SHA1 (GitHub)
let signature = hmac_sha1(b"your-secret-key", b"payload");
```

### Verify Webhook Signature

```rust
use foodshare_crypto::{hmac_sha256, constant_time_compare};

fn verify_stripe_webhook(payload: &[u8], signature: &str, secret: &[u8]) -> bool {
    let expected = hmac_sha256(secret, payload);
    constant_time_compare(expected.as_bytes(), signature.as_bytes())
}
```

### Constant-Time Comparison

Always use constant-time comparison for security-sensitive operations:

```rust
use foodshare_crypto::constant_time_compare;

// Safe: takes the same time regardless of where strings differ
let is_valid = constant_time_compare(b"signature1", b"signature2");
```

## Provider Examples

### Stripe

```rust
use foodshare_crypto::{hmac_sha256, constant_time_compare};

fn verify_stripe(payload: &str, sig_header: &str, secret: &str) -> bool {
    // Stripe sends: t=timestamp,v1=signature
    let parts: Vec<&str> = sig_header.split(',').collect();
    let timestamp = parts[0].strip_prefix("t=").unwrap();
    let signature = parts[1].strip_prefix("v1=").unwrap();
    
    let signed_payload = format!("{}.{}", timestamp, payload);
    let expected = hmac_sha256(secret.as_bytes(), signed_payload.as_bytes());
    
    constant_time_compare(expected.as_bytes(), signature.as_bytes())
}
```

### GitHub

```rust
use foodshare_crypto::{hmac_sha1, constant_time_compare};

fn verify_github(payload: &[u8], signature: &str, secret: &[u8]) -> bool {
    // GitHub sends: sha1=signature
    let sig = signature.strip_prefix("sha1=").unwrap_or(signature);
    let expected = hmac_sha1(secret, payload);
    constant_time_compare(expected.as_bytes(), sig.as_bytes())
}
```

### Meta/Facebook

```rust
use foodshare_crypto::{hmac_sha256, constant_time_compare};

fn verify_meta(payload: &[u8], signature: &str, secret: &[u8]) -> bool {
    // Meta sends: sha256=signature
    let sig = signature.strip_prefix("sha256=").unwrap_or(signature);
    let expected = hmac_sha256(secret, payload);
    constant_time_compare(expected.as_bytes(), sig.as_bytes())
}
```

## Security Considerations

1. **Always use constant-time comparison** - Never use `==` for signature comparison
2. **Keep secrets secure** - Never log or expose webhook secrets
3. **Validate timestamps** - Check webhook timestamp to prevent replay attacks
4. **Use HTTPS** - Always receive webhooks over TLS

## WASM Usage

For browser/Deno usage, see [@foodshare/crypto-wasm](https://www.npmjs.com/package/@foodshare/crypto-wasm).

```typescript
import init, { hmac_sha256_hex, verify_webhook_sha256 } from '@foodshare/crypto-wasm';

await init();
const signature = hmac_sha256_hex('secret', 'payload');
const isValid = verify_webhook_sha256('secret', 'payload', signature);
```

## License

MIT License - see [LICENSE](LICENSE) for details.
