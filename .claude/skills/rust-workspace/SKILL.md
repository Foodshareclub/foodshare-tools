---
name: rust-workspace
description: Rust workspace conventions for foodshare-tools. Use when understanding crate organization, workspace.dependencies, error code patterns, or miette error handling. Covers the 19 library crates and 15 binaries.
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
│   ├── cli/                  # CLI utilities: terminal output
│   ├── app/                  # Cross-platform app: Xcode, Gradle, simulators, emulators
│   ├── web/                  # Web: Next.js security, bundle analysis
│   ├── telemetry/            # Observability: logging, metrics, tracing
│   ├── orchestrator/         # Workspace build and verification orchestration
│   ├── maestro/              # Maestro E2E flows, devices, matrix
│   ├── device/               # iOS simulators and Android emulators
│   ├── wasm/                 # WebAssembly builds for the published crates
│   ├── xccloud/              # Xcode Cloud over the App Store Connect API
│   ├── geo/                  # Geolocation utilities
│   ├── crypto/               # Cryptographic utilities
│   ├── search/               # Search infrastructure
│   ├── compression/          # Compression utilities
│   ├── image/                # Image processing
│   ├── motherduck-sync/      # MotherDuck sync
│   ├── swift-toolchain/      # Swift toolchain management
│   ├── api-client/           # API client
│   ├── quadlet/              # Quadlet validation (a binary that lives under crates/)
├── bins/                     # Binary crates
│   ├── fs-app/               # The cross-platform app CLI
│   ├── lefthook-rs/          # Web CLI binary
│   ├── fs-image/             # Image CLI
│   ├── foodshare-i18n/       # i18n CLI
│   ├── foodshare-swift/      # Swift toolchain CLI
│   ├── fs-auto/              # Headless run-until-green orchestrator
│   ├── fs-tui/               # Live build and test monitor
│   ├── fs-orchestrator/      # HTTP daemon over the orchestrator
│   ├── fs-maestro/           # Maestro E2E runner
│   ├── fs-wasm/              # WebAssembly build engine
│   ├── fs-build/             # Cross-platform build engine
│   ├── fs-sim/               # Simulator runner
│   ├── fs-xccloud/           # Xcode Cloud builds, logs and diagnostics
│   └── fs-verify/            # Full cross-repository verification
```

## Workspace Configuration

```toml
# Cargo.toml (root)
[workspace]
# Resolver 3 makes Cargo honour `rust-version` when it picks dependency
# versions. Resolver 2 does not, and this workspace does not commit a
# Cargo.lock, so a CI run resolved a dependency that declared a newer
# MSRV than the workspace's.
resolver = "3"
members = ["crates/core", "crates/hooks", ...]   # Listed one by one

[workspace.package]
version = "1.4.0"
edition = "2024"
rust-version = "1.88"
```

## Using Workspace Dependencies

In crate `Cargo.toml`:
```toml
[dependencies]
clap.workspace = true                 # Inherits version from workspace
foodshare-core.workspace = true       # Internal dependency, also inherited
```

## Error Code System

| Code Range | Category | Example | Exit |
|------------|----------|---------|------|
| E1xxx | General | E1001 Internal error | 1 |
| E2xxx | IO | E2001 File not found | 1 |
| E3xxx | Configuration | E3002 Parse error | 3 |
| E4xxx | Git | E4001 Not a git repo | 4 |
| E5xxx | Process | E5001 Command not found | 127 |
| E6xxx | Validation | E6001 Invalid input | 2 |
| E7xxx | Security | E7001 Secret detected | 5 |
| E8xxx | Platform | E8001 Xcode error | 1 |

A timeout is 124.

## Error Handling Pattern

The error type is `foodshare_core::error::Error`. It carries an `ErrorCode`,
and the code decides the exit code:

```rust
use foodshare_core::error::{Error, ErrorCode};

let error = Error::new(ErrorCode::SecretDetected, "a token was found");
assert_eq!(error.exit_code(), 5);
assert_eq!(error.coded_message(), "E7001: a token was found");
```

## Common Commands

```bash
cargo build --workspace                    # Build all
cargo test --workspace                     # Test all
cargo clippy --workspace --all-targets -- -D clippy::correctness -D clippy::suspicious
cargo fmt --all                            # Format all
cargo bench --workspace                    # Benchmark all
```

> The clippy gate is the scoped list above, not `-D warnings`. The core crate
> enables `clippy::pedantic` and there are still pedantic findings; the
> scoped list is what CI enforces.
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
