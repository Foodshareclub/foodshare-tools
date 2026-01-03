# Publishing Guide

This document describes how to publish new versions of the foodshare-tools crates.

## Published Packages

### Rust Crates (crates.io)

| Crate | Description |
|-------|-------------|
| [foodshare-geo](https://crates.io/crates/foodshare-geo) | Geospatial utilities for distance calculations |
| [foodshare-crypto](https://crates.io/crates/foodshare-crypto) | Cryptographic utilities for webhook verification |
| [foodshare-search](https://crates.io/crates/foodshare-search) | High-performance fuzzy search |
| [foodshare-compression](https://crates.io/crates/foodshare-compression) | Brotli/Gzip compression utilities |
| [foodshare-image](https://crates.io/crates/foodshare-image) | Image format detection and processing |

### npm Packages (WebAssembly)

| Package | Description |
|---------|-------------|
| [@foodshare/geo-wasm](https://www.npmjs.com/package/@foodshare/geo-wasm) | WASM build for geospatial utilities |
| [@foodshare/crypto-wasm](https://www.npmjs.com/package/@foodshare/crypto-wasm) | WASM build for crypto utilities |
| [@foodshare/search-wasm](https://www.npmjs.com/package/@foodshare/search-wasm) | WASM build for fuzzy search |
| [@foodshare/compression-wasm](https://www.npmjs.com/package/@foodshare/compression-wasm) | WASM build for compression |

## Publishing to crates.io

### Automated (Recommended)

We use [Trusted Publishing](https://blog.rust-lang.org/2023/11/09/crates-io-trusted-publishing.html) for secure, token-free publishing from GitHub Actions.

#### Publish via Git Tag

```bash
# Update version in Cargo.toml files
# Commit the changes
git add .
git commit -m "Bump version to 1.3.2"

# Create and push tag
git tag v1.3.2
git push origin v1.3.2
```

The `publish.yml` workflow will automatically publish all crates.

#### Manual Trigger

1. Go to [Actions → Publish to crates.io](https://github.com/Foodshareclub/foodshare-tools/actions/workflows/publish.yml)
2. Click "Run workflow"
3. Select which crate to publish (or "all")
4. Click "Run workflow"

### Manual Publishing

If needed, you can publish manually with a crates.io API token:

```bash
# Login (one-time)
cargo login

# Publish individual crate
cargo publish -p foodshare-geo

# Or publish all (in dependency order)
cargo publish -p foodshare-geo
cargo publish -p foodshare-crypto
cargo publish -p foodshare-search
cargo publish -p foodshare-compression
cargo publish -p foodshare-image
```

## Publishing npm Packages

### Build WASM Packages

```bash
# Install wasm-pack if needed
cargo install wasm-pack

# Build each package
wasm-pack build crates/geo --target web --out-dir ../../packages/geo-wasm
wasm-pack build crates/crypto --target web --out-dir ../../packages/crypto-wasm --features wasm
wasm-pack build crates/search --target web --out-dir ../../packages/search-wasm --features wasm
wasm-pack build crates/compression --target web --out-dir ../../packages/compression-wasm --features wasm
```

### Publish to npm

```bash
# Login to npm (one-time)
npm login

# Publish each package
cd packages/geo-wasm && npm publish --access public
cd ../crypto-wasm && npm publish --access public
cd ../search-wasm && npm publish --access public
cd ../compression-wasm && npm publish --access public
```

Note: If you have 2FA enabled, you'll need to provide an OTP for each publish:

```bash
npm publish --access public --otp=123456
```

## Version Bumping

All crates share the same version via workspace inheritance. To bump versions:

1. Update `version` in the root `Cargo.toml`:
   ```toml
   [workspace.package]
   version = "1.3.2"
   ```

2. Update `version` in each `packages/*/package.json`

3. Commit and tag:
   ```bash
   git add .
   git commit -m "Bump version to 1.3.2"
   git tag v1.3.2
   git push && git push --tags
   ```

## Trusted Publishing Setup

Trusted publishers are configured in crates.io settings for each crate:

| Setting | Value |
|---------|-------|
| Repository owner | `Foodshareclub` |
| Repository name | `foodshare-tools` |
| Workflow filename | `publish.yml` |
| Environment name | `crates-io` |

The `crates-io` environment should be configured in [GitHub repository settings](https://github.com/Foodshareclub/foodshare-tools/settings/environments).

## Pre-publish Checklist

- [ ] All tests pass: `cargo test --all`
- [ ] Clippy is happy: `cargo clippy --all`
- [ ] Version bumped in `Cargo.toml`
- [ ] Version bumped in `package.json` files
- [ ] CHANGELOG updated (if applicable)
- [ ] README examples are up to date
