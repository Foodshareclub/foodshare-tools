# Getting Started

## Prerequisites

- Rust 1.85 or later
- Git 2.30+
- Platform-specific tools (see below)

## Installation

### From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/Foodshareclub/foodshare-tools.git
cd foodshare-tools

# Build all binaries
cargo build --release

# Binaries are in target/release/
ls target/release/foodshare-*
```

### Install Globally

```bash
# iOS tools
cargo install --path bins/fs-ios

# Android tools
cargo install --path bins/fs-android

# Web tools (lefthook-rs)
cargo install --path bins/lefthook-rs

# Image tools
cargo install --path bins/fs-image
```

### Quick Install Script

```bash
curl -sSL https://raw.githubusercontent.com/Foodshareclub/foodshare-tools/main/install.sh | bash
```

## Platform Setup

### iOS Development

Required tools:
- Xcode 15.2+
- SwiftFormat
- SwiftLint

```bash
# Verify environment
fs-ios doctor

# Expected output:
# ✓ git (2.43.0)
# ✓ xcodebuild (15.2)
# ✓ swift (5.9.2)
# ✓ swiftformat
# ✓ swiftlint
```

### Android Development

Required tools:
- Android Studio / SDK
- Kotlin compiler
- Gradle

```bash
# Verify environment
fs-android doctor
```

### Web Development

Required tools:
- Node.js 18+
- bun

```bash
# Verify environment
lefthook-rs doctor
```

## Project Integration

### 1. Add Configuration

Create `.foodshare-hooks.toml` in your project root:

```toml
[commit_msg]
types = ["feat", "fix", "docs", "style", "refactor", "test", "chore", "ci", "perf"]
max_subject_length = 72

[secrets]
exclude_files = ["*.test.ts", "*.mock.ts"]

[migrations]
directory = "supabase/migrations"
```

### 2. Configure Lefthook

Add to `lefthook.yml`:

```yaml
pre-commit:
  parallel: true
  commands:
    secrets:
      run: fs-ios secrets  # or fs-android, lefthook-rs

commit-msg:
  commands:
    validate:
      run: fs-ios commit-msg {1}

pre-push:
  commands:
    checks:
      run: fs-ios pre-push
```

### 3. Install Git Hooks

```bash
lefthook install
```

## First Commands

```bash
# Check for secrets in staged files
fs-ios secrets

# Validate a commit message
echo "feat: add new feature" | fs-ios commit-msg -

# Run all pre-push checks
fs-ios pre-push

# Format Swift code (iOS)
fs-ios format --staged
```

## Next Steps

- [CLI Reference](./cli-reference.md) - Full command documentation
- [Configuration](./configuration.md) - All configuration options
- [Architecture](./architecture.md) - How the tools are structured
