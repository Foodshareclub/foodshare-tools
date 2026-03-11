# Foodshare Tools

[![CI](https://github.com/Foodshareclub/foodshare-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/Foodshareclub/foodshare-tools/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![codecov](https://codecov.io/gh/Foodshareclub/foodshare-tools/branch/main/graph/badge.svg)](https://codecov.io/gh/Foodshareclub/foodshare-tools)

Enterprise-grade Rust CLI workspace for git hooks, code quality, and development tooling across all Foodshare platforms.

## Features

- 🚀 **Fast**: Written in Rust for maximum performance (~10x faster than shell scripts)
- 🔒 **Secure**: Built-in secret scanning with 15+ patterns for API keys, tokens, and credentials
- 📊 **Observable**: Structured logging, metrics, and health checks
- 🔧 **Configurable**: TOML-based configuration with validation
- 🎯 **Unified**: Single codebase for all platforms with platform-specific extensions
- ✅ **Tested**: Comprehensive test suite with property-based testing

## Quick Start

```bash
git clone https://github.com/Foodshareclub/foodshare-tools.git
cd foodshare-tools

# Build all binaries
cargo build --release

# Or install individually
cargo install --path bins/fs-app
cargo install --path bins/lefthook-rs
```

## Architecture

```
foodshare-tools/
├── crates/                    # Library crates (16)
│   ├── core/                  # Shared: git, file scanning, process, health
│   ├── hooks/                 # Git hooks: commit-msg, secrets, migrations
│   ├── cli/                   # CLI utilities: terminal output, progress
│   ├── ios/                   # iOS: Xcode, simulators, Swift tools
│   ├── android/               # Android: Gradle, emulators, Kotlin
│   ├── web/                   # Web: Next.js security, bundle analysis
│   ├── telemetry/             # Observability: logging, metrics, tracing
│   ├── geo/                   # Geospatial utilities
│   ├── crypto/                # HMAC, constant-time comparison
│   ├── search/                # Search utilities
│   ├── compression/           # Brotli compression
│   ├── image/                 # Image processing (JPEG, PNG, WebP, GIF)
│   ├── swift-toolchain/       # Swift version management
│   ├── api-client/            # API client utilities
│   ├── migrate/               # Migration tooling
│   └── motherduck-sync/       # MotherDuck data sync
├── bins/                      # Binary crates (7)
│   ├── fs-app/         # Cross-platform CLI
│   ├── lefthook-rs/           # Web CLI (git hooks)
│   ├── fs-image/              # Image processing CLI
│   ├── foodshare-i18n/        # Internationalization CLI
│   ├── foodshare-swift/       # Swift toolchain CLI
│   └── foodshare-migrate/     # Migration CLI
├── scripts/                   # Utility scripts
├── tests/                     # Integration tests
├── docs/                      # Documentation
└── .github/workflows/
    ├── ci.yml                 # CI: lint, test (3 OS), coverage, build, docs
    ├── publish.yml            # Publish crates to crates.io
    └── release.yml            # Build release binaries + GitHub Release
```

## Usage

### Shared Commands (all platforms)

```bash
# Validate commit message format
<binary> commit-msg .git/COMMIT_MSG

# Scan for secrets in staged files
<binary> secrets

# Check Supabase migrations
<binary> migrations --dir supabase/migrations

# Run pre-push checks
<binary> pre-push
```

### iOS (used with foodshare-app)

```bash
fs-app format --staged        # Format Swift code
fs-app lint --strict          # Lint with strict mode
fs-app build --configuration release
fs-app simulator list         # Manage simulators
fs-app project status         # Xcode project analysis
fs-app doctor                 # Environment check
```

### Android (used with foodshare-app)

```bash
fs-app format --lang kotlin
fs-app lint
fs-app emulator list
fs-app emulator boot pixel_7
```

### Web (used with foodshare-web)

```bash
lefthook-rs security                 # Security checks
lefthook-rs nextjs-security          # Next.js specific
lefthook-rs bundle-size --threshold 500kb
lefthook-rs conventional-commit .git/COMMIT_MSG
```

## Related Repositories

| Repository | Purpose |
|------------|---------|
| [`foodshare-app`](https://github.com/Foodshareclub/foodshare-app) | Unified cross-platform app (Skip Fuse: iOS + Android) |
| [`foodshare-web`](https://github.com/Foodshareclub/foodshare-web) | Next.js 16 web app |
| [`foodshare-backend`](https://github.com/Foodshareclub/foodshare-backend) | Self-hosted Supabase backend |
| [`foodshare-runner`](https://github.com/Foodshareclub/foodshare-runner) | GitHub Actions self-hosted runner |

## Development

```bash
cargo test --workspace              # Run all tests
cargo llvm-cov --workspace          # Coverage report
cargo bench --workspace             # Run benchmarks
cargo fmt --all -- --check          # Check formatting
cargo clippy --workspace --all-targets -- -D warnings  # Lint
```

## Error Codes

| Code Range | Category | Example |
|------------|----------|---------|
| E1xxx | General | E1001 Internal error |
| E2xxx | IO | E2001 File not found |
| E3xxx | Configuration | E3002 Parse error |
| E4xxx | Git | E4001 Not a git repo |
| E5xxx | Process | E5001 Command not found |
| E6xxx | Validation | E6001 Invalid input |
| E7xxx | Security | E7001 Secret detected |
| E8xxx | Platform | E8001 Xcode error |

## License

MIT — see [LICENSE](LICENSE) for details.
