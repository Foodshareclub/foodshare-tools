# foodshare-crypto

Cryptographic utilities for HMAC signatures and webhook verification.

## Installation

```toml
[dependencies]
foodshare-crypto = "1.4"
```

## Features

- HMAC-SHA256 signatures
- Webhook payload verification
- Constant-time comparison
- WASM support

## Usage

### HMAC Signatures

```rust
use foodshare_crypto::{hmac_sha256, verify_signature};

let secret = b"my-secret-key";
let payload = b"webhook payload";

// Generate signature
let signature = hmac_sha256(secret, payload);

// Verify signature (constant-time)
let valid = verify_signature(secret, payload, &signature);
```

### Webhook Verification

```rust
use foodshare_crypto::webhook;

// Verify Stripe webhook
let valid = webhook::verify_stripe(
    &payload,
    &signature_header,
    &webhook_secret,
)?;

// Verify GitHub webhook
let valid = webhook::verify_github(
    &payload,
    &signature_header,
    &webhook_secret,
)?;
```

## WASM Usage

```typescript
import init, { hmac_sha256, verify_signature } from '@foodshare/crypto-wasm';

await init();

const signature = hmac_sha256('secret', 'payload');
const valid = verify_signature('secret', 'payload', signature);
```

## Security

- Uses constant-time comparison to prevent timing attacks
- No unsafe code
- Audited dependencies (hmac, sha2, subtle)

## Links

- [crates.io](https://crates.io/crates/foodshare-crypto)
- [npm](https://www.npmjs.com/package/@foodshare/crypto-wasm)
