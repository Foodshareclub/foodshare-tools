---
name: rust-workspace
description: Rust workspace conventions for foodshare-tools. Use when understanding crate organization, workspace.dependencies, error code patterns, or miette error handling. Covers the 18+ crate workspace structure.
---

<objective>
Follow workspace conventions for the foodshare-tools Rust monorepo including crate organization, shared dependencies, error codes, and consistent patterns across all crates.
</objective>

<essential_principles>
## Workspace Structure

```
foodshare-tools/
├── Cargo.toml                # Workspace root (members, workspace.dependencies)
├── crates/                   # Library crates
│   ├── core/                 # Shared: git, file scanning, process, health checks
│   ├── hooks/                # Git hooks: commit-msg, secrets, migrations
│   ├── cli/                  # CLI utilities: terminal output, progress
│   ├── ios/                  # iOS: Xcode, simulators, Swift tools
│   ├── android/              # Android: Gradle, emulators, Kotlin, Swift-Android
│   ├── web/                  # Web: Next.js security, bundle analysis
│   ├── telemetry/            # Observability: logging, metrics, tracing
│   ├── geo/                  # Geolocation utilities
│   ├── crypto/               # Cryptographic utilities
│   ├── search/               # Search infrastructure
│   ├── compression/          # Compression utilities
│   ├── image/                # Image processing
│   ├── motherduck-sync/      # MotherDuck sync
│   ├── swift-toolchain/      # Swift toolchain management
│   ├── api-client/           # API client
│   └── migrate/              # Migration utilities
├── bins/                     # Binary crates
│   ├── fs-ios/        # iOS CLI binary
│   ├── fs-android/    # Android CLI binary
│   ├── lefthook-rs/          # Web CLI binary
│   ├── fs-image/             # Image CLI
│   ├── foodshare-i18n/       # i18n CLI
│   ├── foodshare-swift/      # Swift toolchain CLI
│   └── foodshare-migrate/    # Migration CLI
```

## Workspace Configuration

```toml
# Cargo.toml (root)
[workspace]
resolver = "2"
members = ["crates/*", "bins/*"]

[workspace.package]
version = "1.4.0"
edition = "2024"
rust-version = "1.85"

[workspace.dependencies]
clap = { version = "4.4", features = ["derive", "env", "wrap_help", "string"] }
# ... shared dependencies
```

## Using Workspace Dependencies

In crate `Cargo.toml`:
```toml
[dependencies]
clap = { workspace = true }         # Inherits version from workspace
foodshare-core = { path = "../core" }  # Internal dependency
```

## Error Code System

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

## Error Handling Pattern

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("E2001: File not found: {path}")]
    FileNotFound { path: String },

    #[error("E4001: Not a git repository")]
    NotGitRepo,

    #[error("E5001: Command not found: {command}")]
    CommandNotFound { command: String },
}
```

## Common Commands

```bash
cargo build --workspace                    # Build all
cargo test --workspace                     # Test all
cargo clippy --workspace --all-targets -- -D warnings  # Lint all
cargo fmt --all                            # Format all
cargo bench --workspace                    # Benchmark all
```
</essential_principles>

<success_criteria>
Workspace conventions followed when:
- [ ] New crates added to workspace.members
- [ ] Shared deps use workspace.dependencies
- [ ] Error codes follow the E{category}xxx pattern
- [ ] Crates in correct directory (crates/ for libs, bins/ for binaries)
- [ ] Workspace package metadata inherited
- [ ] `cargo check --workspace --all-targets` passes
</success_criteria>
