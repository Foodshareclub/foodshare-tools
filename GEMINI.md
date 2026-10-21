# Foodshare Tools

Rust CLI workspace for git hooks, code quality, and development tooling across iOS, Android, and Web.

**Version:** 1.4.0 | **Edition:** 2024 | **MSRV:** 1.88 (via `Cargo.toml`; no `rust-toolchain.toml`) | **License:** MIT

## Toolchain

The Rust in this repository is the toolchain; Bun runs everything around it —
the package manager, the script runner, the TypeScript executor, the test
runner and the type checker. Type checking is `tsrs`, the Rust port of the
TypeScript 7 compiler, with `tsc` on TypeScript 7.1 kept as a parity gate.

Node.js is not part of it. `bun scripts/check-no-node.ts` fails the build if
it creeps back in. `bunx` is the one exception to `[run] bun = true` — it
resolves a package's own shebang — so every `bunx` call here passes `--bun`.

## Commands

| Command                                                 | Purpose            |
| ------------------------------------------------------- | ------------------ |
| `cargo build --release --workspace`                     | Build all binaries |
| `cargo test --workspace`                                | Run all tests      |
| `cargo fmt --all -- --check`                            | Check formatting   |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint               |
| `cargo llvm-cov --workspace`                            | Coverage report    |
| `cargo bench --workspace`                               | Run benchmarks     |

## Project Structure

```
foodshare-tools/
├── crates/                    # Library crates (21)
│   ├── core/                  # Shared: git, file scanning, process, health
│   ├── hooks/                 # Git hooks: commit-msg, secrets, migrations
│   ├── cli/                   # CLI utilities: terminal output, progress
│   ├── app/                   # Cross-platform app support (iOS + Android)
│   ├── web/                   # Web: Next.js security, bundle analysis
│   ├── telemetry/             # Observability: logging, metrics, tracing
│   ├── orchestrator/          # Workspace build and verification orchestration
│   ├── maestro/               # Maestro E2E flows, devices, matrix
│   ├── wasm/                  # WebAssembly builds for the published crates
│   ├── xccloud/               # Xcode Cloud over the App Store Connect API
│   ├── device/                # iOS simulators and Android emulators
│   ├── geo/                   # Geospatial utilities
│   ├── crypto/                # HMAC, constant-time comparison
│   ├── search/                # Search utilities
│   ├── compression/           # Brotli compression
│   ├── image/                 # Image processing (JPEG, PNG, WebP, GIF)
│   ├── swift-toolchain/       # Swift version management
│   ├── api-client/            # API client utilities
│   ├── migrate/               # Migration tooling
│   └── motherduck-sync/       # MotherDuck data sync (placeholder submodule)
├── bins/                      # Binary crates (15)
│   ├── fs-app/         # Cross-platform CLI
│   ├── lefthook-rs/           # Web CLI (git hooks)
│   ├── fs-image/              # Image processing CLI
│   ├── foodshare-i18n/        # Internationalization CLI
│   ├── foodshare-swift/       # Swift toolchain CLI
│   ├── foodshare-migrate/     # Migration CLI
│   ├── fs-auto/               # Headless run-until-green orchestrator
│   ├── fs-tui/                # Live build and test monitor
│   ├── fs-orchestrator/       # HTTP daemon over the orchestrator
│   ├── fs-maestro/            # Maestro E2E runner
│   ├── fs-wasm/               # WebAssembly build engine
│   ├── fs-build/              # Cross-platform build engine
│   ├── fs-sim/                # Simulator runner
│   ├── fs-xccloud/            # Xcode Cloud builds, logs and diagnostics
│   └── fs-verify/             # Full cross-repository verification
├── scripts/                   # Utility scripts
├── tests/                     # Integration tests
├── docs/                      # Documentation
│   └── adr/                   # Architecture decision records
├── packages/                  # Additional packages
├── .github/workflows/
│   ├── ci.yml                 # CI: lint, security audit (cargo-audit), test (3 OS), coverage, build (4 targets), docs
│   ├── publish.yml            # Publish crates to crates.io
│   └── release.yml            # Build release binaries + GitHub Release
└── Cargo.toml                 # Workspace manifest
```

## CI/CD

Three workflows:

- **`ci.yml`** — Runs on push/PR: lint → security audit (cargo-audit) → test (3 OS) → coverage → build (4 targets) → docs
- **`publish.yml`** — Triggered by `v*` tags or manual dispatch: publishes crates to crates.io
- **`release.yml`** — Triggered by `v*` tags: builds binaries, packages tarballs, creates GitHub Release

## Key Crates

| Crate                    | Purpose                                                   |
| ------------------------ | --------------------------------------------------------- |
| `foodshare-core`         | Git operations, file scanning, process management         |
| `foodshare-hooks`        | Commit message validation, secret scanning (15+ patterns) |
| `foodshare-cli`          | Terminal output, progress bars, colored output            |
| `fs-app`                 | Cross-platform (iOS, Android) CLI and project management  |
| `foodshare-web`          | Next.js security checks, bundle analysis                  |
| `foodshare-telemetry`    | Structured logging, Prometheus metrics                     |
| `foodshare-orchestrator` | Workspace build and verification orchestration            |
| `foodshare-maestro`    | Maestro E2E flows, syntax checks, device matrix            |
| `foodshare-wasm`       | WebAssembly builds for the published crates                |
| `foodshare-xccloud`    | Xcode Cloud over the App Store Connect API                 |

## Orchestration

The workspace verification lives in `foodshare-orchestrator`, a library
with no HTTP dependency. Three front-ends drive it: `fs-auto` (headless),
`fs-tui` (live monitor) and `fs-orchestrator` (the axum daemon). The
daemon serves the same probe contract as the backend's `job-worker`.

The daemon is the only crate that depends on axum, tower or tower-http,
at the same versions the backend resolves. It is built with the
`release-service` profile, which inherits `release` but unwinds: under
`panic = "abort"` one panicking handler would take the process down.

The decision record is in `docs/adr/0001-rust-orchestrator-and-axum.md`.

## Important Rules

1. **Rust edition 2024** — All crates use `edition = "2024"` via workspace inheritance.
2. **No `deno.lock`** — Do not commit lock files for non-Rust tooling.
3. **Workspace dependencies** — All dependency versions are centralized in the root `Cargo.toml`.
4. **Release profile** — Uses `lto = "thin"`, `codegen-units = 1`, `strip = true` for optimized binaries.
5. **Cross-platform** — CI tests on `ubuntu-latest`, `macos-latest`, and `macos-14` (ARM64).
6. **Error codes** — All errors use structured codes (E1xxx–E8xxx) for programmatic handling.
