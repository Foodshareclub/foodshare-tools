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

### One-time passwords

```rust
use foodshare_crypto::totp;

// The code for a secret, right now
let code = foodshare_crypto::generate_totp(secret, 30, 6)?;

// Whether a code is the one for a secret
let valid = foodshare_crypto::verify_totp(secret, &code, 30, 6)?;

// The URI an authenticator application reads
let uri = foodshare_crypto::build_otpauth_uri("Foodshare", secret, 30, 6)?;
```

### Constant-time comparison

```rust
use foodshare_crypto::constant_time_compare;

let equal = constant_time_compare(provided.as_bytes(), expected.as_bytes());
```

## WASM Usage

```typescript
import init, { hmac_sha256_hex, verify_webhook_sha256 } from '@foodshare/crypto-wasm';

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
