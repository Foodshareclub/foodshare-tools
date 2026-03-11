# Foodshare Tools

Rust CLI workspace for git hooks, code quality, and development tooling across iOS, Android, and Web.

**Version:** 1.4.0 | **Edition:** 2024 | **MSRV:** 1.85 | **License:** MIT

## Commands

| Command | Purpose |
|---------|---------|
| `cargo build --release --workspace` | Build all binaries |
| `cargo test --workspace` | Run all tests |
| `cargo fmt --all -- --check` | Check formatting |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint |
| `cargo llvm-cov --workspace` | Coverage report |
| `cargo bench --workspace` | Run benchmarks |

## Project Structure

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
├── packages/                  # Additional packages
├── .github/workflows/
│   ├── ci.yml                 # CI: lint, test, build, coverage, docs
│   ├── publish.yml            # Publish crates to crates.io
│   └── release.yml            # Build release binaries + GitHub Release
├── Cargo.toml                 # Workspace manifest
└── Cargo.lock
```

## CI/CD

Three workflows:
- **`ci.yml`** — Runs on push/PR: lint → test (3 OS) → coverage → build (3 targets) → docs → MSRV check
- **`publish.yml`** — Triggered by `v*` tags or manual dispatch: publishes crates to crates.io
- **`release.yml`** — Triggered by `v*` tags: builds binaries, packages tarballs, creates GitHub Release

## Key Crates

| Crate | Purpose |
|-------|---------|
| `foodshare-core` | Git operations, file scanning, process management |
| `foodshare-hooks` | Commit message validation, secret scanning (15+ patterns) |
| `foodshare-cli` | Terminal output, progress bars, colored output |
| `fs-app` | Cross-platform (iOS, Android) CLI and project management |
| `foodshare-web` | Next.js security checks, bundle analysis |
| `foodshare-telemetry` | Structured logging, Prometheus metrics |
| `foodshare-geo` | Geospatial utilities |
| `foodshare-crypto` | HMAC signing, constant-time comparison |

## Important Rules

1. **Rust edition 2024** — All crates use `edition = "2024"` via workspace inheritance.
2. **No `deno.lock`** — Do not commit lock files for non-Rust tooling.
3. **Workspace dependencies** — All dependency versions are centralized in the root `Cargo.toml`.
4. **Release profile** — Uses `lto = "thin"`, `codegen-units = 1`, `strip = true` for optimized binaries.
5. **Cross-platform** — CI tests on `ubuntu-latest`, `macos-latest`, and `macos-14` (ARM64).
6. **Error codes** — All errors use structured codes (E1xxx–E8xxx) for programmatic handling.
