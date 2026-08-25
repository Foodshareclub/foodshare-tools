# Getting Started

## Prerequisites

- Rust 1.88 or later
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
ls target/release/{fs-app,lefthook-rs,fs-image} foodshare-*
```

### Install Globally

```bash
# iOS tools
cargo install --path bins/fs-app

# Android tools
cargo install --path bins/fs-app

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
fs-app doctor

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
fs-app doctor
```

### Web Development

Required tools:

- Node.js 18+
- bun

```bash
# Available subcommands
lefthook-rs security              # Secret scanning on staged files
lefthook-rs conventional-commit   # Commit message validation
lefthook-rs protected-branch      # Protected branch checks
lefthook-rs large-files           # Large file detection
lefthook-rs nextjs-security       # Next.js-specific security checks
lefthook-rs accessibility         # Accessibility checks
lefthook-rs bundle-size           # Bundle size analysis
lefthook-rs pre-commit            # Full pre-commit gate
```

## Project Integration

### 1. Add Configuration

Create `.foodshare-hooks.toml` in your project root:

```toml
[commit_msg]
types = ["feat", "fix", "docs", "style", "refactor", "test", "chore", "ci", "perf"]
max_length = 72

[secrets]
exclude_files = ["*.test.ts", "*.mock.ts"]
```

### 2. Wire Up Lefthook (Centralized)

Repos use the centralized `lefthook.yml` from foodshare-tools via symlink, with gates calling the built `lefthook-rs` binary through `FOODSHARE_HOOKS_BIN` (default: `../foodshare-tools/target/{release,debug}/lefthook-rs`):

```bash
# Symlink the centralized hook config into your repo
ln -s ../foodshare-tools/lefthook.yml lefthook.yml

# Point lefthook at your locally built lefthook-rs binary
export FOODSHARE_HOOKS_BIN=../foodshare-tools/target/release/lefthook-rs
```

The priority-tiered gates run:

- **pre-commit** — staged-file secret scan (`lefthook-rs security {staged_files}`), `nextjs-security`, oxlint/biome/prettier, and bun tests on staged files
- **pre-push** — typecheck (`tsc`/`deno check`), Next.js build, full test suite, i18n-sync-check, supabase-secret-check, package audit, protected-branch, large-files
- **commit-msg** — conventional-commit validation via `lefthook-rs conventional-commit`

### 3. Install Git Hooks

```bash
lefthook install
```

## First Commands

```bash
# Check for secrets in staged files
fs-app secrets

# Validate a commit message
echo "feat: add new feature" | fs-app commit-msg -

# Run all pre-push checks
fs-app pre-push

# Format Swift code (iOS)
fs-app format --staged
```

## Next Steps

- [CLI Reference](./cli-reference.md) - Full command documentation
- [Configuration](./configuration.md) - All configuration options
- [Architecture](./architecture.md) - How the tools are structured
