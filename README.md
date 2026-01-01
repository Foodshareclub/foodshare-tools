# Foodshare Tools

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

Unified Rust CLI for git hooks and development tools across all Foodshare platforms (iOS, Android, Web).

## Features

- **Fast**: Written in Rust for maximum performance
- **Unified**: Single codebase for all platforms
- **Modular**: Use only what you need via feature flags
- **Extensible**: Easy to add new checks and tools

## Quick Start

```bash
# Install from source
cargo install --path bins/foodshare-ios
cargo install --path bins/foodshare-android
cargo install --path bins/lefthook-rs

# Or build all
cargo build --release --workspace
```

## Architecture

```
foodshare-tools/
├── crates/
│   ├── core/       # Shared core: git, file scanning, process execution
│   ├── hooks/      # Git hooks: commit-msg, secrets, migrations, pre-push
│   ├── cli/        # CLI utilities: args parsing, terminal output
│   ├── ios/        # iOS-specific: Xcode, simulators, Swift tools
│   ├── android/    # Android-specific: Gradle, emulators, Kotlin tools
│   └── web/        # Web-specific: Next.js security, bundle analysis
├── bins/
│   ├── foodshare-ios/      # iOS CLI binary
│   ├── foodshare-android/  # Android CLI binary
│   └── lefthook-rs/        # Web CLI binary
```

## Installation

```bash
# Build all binaries
cargo build --release

# Build specific platform
cargo build --release -p foodshare-ios-cli
cargo build --release -p foodshare-android-cli
cargo build --release -p lefthook-rs
```

## Usage

Each platform has its own binary with platform-specific commands plus shared hooks:

### Shared Commands (all platforms)
- `commit-msg` - Validate conventional commit format
- `secrets` - Scan for sensitive data
- `migrations` - Check Supabase migrations
- `pre-push` - Run pre-push checks

### iOS
```bash
foodshare-ios format --staged
foodshare-ios lint --strict
foodshare-ios build --configuration release
foodshare-ios simulator list
```

### Android
```bash
foodshare-android format --lang kotlin
foodshare-android lint --skip-swift
foodshare-android swift-build --target arm64
foodshare-android emulator boot pixel_7
```

### Web
```bash
lefthook-rs security
lefthook-rs nextjs-security
lefthook-rs bundle-size
```

## Development

```bash
# Run tests
cargo test --workspace

# Run benchmarks
cargo bench --workspace

# Check all targets
cargo check --workspace --all-targets
```

## Configuration

Each project uses `.foodshare-hooks.toml` for platform-specific settings.
Shared settings (commit types, secret patterns) are embedded in the core crate.

## License

MIT
