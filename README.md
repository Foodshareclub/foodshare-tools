# Foodshare Tools

[![CI](https://github.com/Foodshareclub/foodshare-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/Foodshareclub/foodshare-tools/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![codecov](https://codecov.io/gh/Foodshareclub/foodshare-tools/branch/main/graph/badge.svg)](https://codecov.io/gh/Foodshareclub/foodshare-tools)

Enterprise-grade Rust CLI for git hooks and development tools across all Foodshare platforms (iOS, Android, Web).

## Features

- 🚀 **Fast**: Written in Rust for maximum performance (~10x faster than shell scripts)
- 🔒 **Secure**: Built-in secret scanning with 15+ patterns for API keys, tokens, and credentials
- 📊 **Observable**: Structured logging, metrics, and health checks
- 🔧 **Configurable**: TOML-based configuration with validation
- 🎯 **Unified**: Single codebase for all platforms with platform-specific extensions
- ✅ **Tested**: Comprehensive test suite with property-based testing

## Quick Start

```bash
# Clone the repository
git clone https://github.com/Foodshareclub/foodshare-tools.git
cd foodshare-tools

# Build all binaries
cargo build --release

# Or install globally
cargo install --path bins/foodshare-ios
cargo install --path bins/foodshare-android
cargo install --path bins/lefthook-rs
```

## Architecture

```
foodshare-tools/
├── crates/
│   ├── core/         # Shared: git, file scanning, process, health checks
│   ├── hooks/        # Git hooks: commit-msg, secrets, migrations
│   ├── cli/          # CLI utilities: terminal output, progress
│   ├── ios/          # iOS: Xcode, simulators, Swift tools, xcodeproj
│   ├── android/      # Android: Gradle, emulators, Kotlin, Swift-Android
│   ├── web/          # Web: Next.js security, bundle analysis
│   └── telemetry/    # Observability: logging, metrics, tracing
├── bins/
│   ├── foodshare-ios/      # iOS CLI binary
│   ├── foodshare-android/  # Android CLI binary
│   └── lefthook-rs/        # Web CLI binary
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

### iOS

```bash
# Format Swift code
foodshare-ios format --staged

# Lint with strict mode
foodshare-ios lint --strict

# Build project
foodshare-ios build --configuration release

# Manage simulators
foodshare-ios simulator list
foodshare-ios simulator boot --device "iPhone 15 Pro"

# Xcode project analysis
foodshare-ios project status
foodshare-ios project missing    # Files on disk not in project
foodshare-ios project broken     # Broken file references

# Environment check
foodshare-ios doctor
```

### Android

```bash
# Format Kotlin code
foodshare-android format --lang kotlin

# Lint code
foodshare-android lint

# Build Swift core for Android
foodshare-android swift-core build --target arm64
foodshare-android swift-core copy --output app/libs

# Manage emulators
foodshare-android emulator list
foodshare-android emulator boot pixel_7
```

### Web

```bash
# Security checks
lefthook-rs security
lefthook-rs nextjs-security

# Bundle size analysis
lefthook-rs bundle-size --threshold 500kb

# Conventional commit validation
lefthook-rs conventional-commit .git/COMMIT_MSG
```

## Configuration

Create `.foodshare-hooks.toml` in your project root:

```toml
[commit_msg]
types = ["feat", "fix", "docs", "style", "refactor", "test", "chore", "ci", "perf"]
max_subject_length = 72
require_scope = false

[secrets]
exclude_files = ["*.test.ts", "*.spec.ts", "*.mock.ts"]
exclude_patterns = ["EXAMPLE_", "PLACEHOLDER_"]

[migrations]
directory = "supabase/migrations"
require_down = true
check_naming = true
```

## Health Checks

Run environment diagnostics:

```bash
# iOS environment
foodshare-ios doctor --json

# Output:
{
  "status": "healthy",
  "checks": [
    {"name": "git", "status": "healthy", "version": "2.43.0"},
    {"name": "xcodebuild", "status": "healthy", "version": "15.2"},
    {"name": "swift", "status": "healthy", "version": "5.9.2"},
    {"name": "swiftformat", "status": "healthy"},
    {"name": "swiftlint", "status": "healthy"}
  ]
}
```

## Integration with Lefthook

Add to your `lefthook.yml`:

```yaml
pre-commit:
  parallel: true
  commands:
    format:
      glob: "*.swift"
      run: foodshare-ios format --staged
    secrets:
      run: foodshare-ios secrets

commit-msg:
  commands:
    validate:
      run: foodshare-ios commit-msg {1}

pre-push:
  commands:
    checks:
      run: foodshare-ios pre-push --fail-fast
```

## Development

```bash
# Run all tests
cargo test --workspace

# Run tests with coverage
cargo llvm-cov --workspace

# Run benchmarks
cargo bench --workspace

# Check all targets
cargo check --workspace --all-targets

# Format code
cargo fmt --all

# Lint code
cargo clippy --workspace --all-targets -- -D warnings
```

## Error Codes

All errors include structured codes for programmatic handling:

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

## Performance

Benchmarks on Apple M1 Pro:

| Operation | Time |
|-----------|------|
| Secret scan (100 files) | ~15ms |
| Commit message validation | ~1ms |
| Format check (staged) | ~50ms |
| Full pre-push suite | ~2s |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT - see [LICENSE](LICENSE) for details.

## Support

- 📖 [Documentation](https://github.com/Foodshareclub/foodshare-tools/wiki)
- 🐛 [Issue Tracker](https://github.com/Foodshareclub/foodshare-tools/issues)
- 💬 [Discussions](https://github.com/Foodshareclub/foodshare-tools/discussions)
