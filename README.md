# Foodshare Tools

[![CI](https://github.com/Foodshareclub/foodshare-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/Foodshareclub/foodshare-tools/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org/)
[![codecov](https://codecov.io/gh/Foodshareclub/foodshare-tools/branch/main/graph/badge.svg)](https://codecov.io/gh/Foodshareclub/foodshare-tools)

Enterprise-grade Rust CLI workspace for git hooks, code quality, and development tooling across all Foodshare platforms.

## Features

- 🚀 **Fast**: Fast single-binary Rust tooling
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

> **Note:** `motherduck-sync` is a placeholder submodule — a library stub only; its CLI is planned (see the [submodule repo](https://github.com/Foodshareclub/motherduck-sync)).

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
├── packages/                  # Additional packages (WASM bindings, shared types)
└── .github/workflows/
    ├── ci.yml                 # CI: lint, security audit, test (3 OS), coverage, build, docs
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

### Orchestration

The workspace verification — Rust, WASM, Web, Backend, Mobile — lives in
`foodshare-orchestrator`, with three front-ends over it.

```bash
fs-auto                              # Headless run-until-green, build, clean
fs-tui                               # Live monitor, interactive
fs-tui verify                        # The whole plan, without the TUI
fs-orchestrator run --domain rust    # One domain from the command line
fs-orchestrator serve                # The HTTP daemon
```

The daemon serves the same probe contract as the backend's `job-worker`,
so one Prometheus covers both services. The probe routes are open; the
control routes that start builds take a token, the way the backend's
job routes do.

| Route                | What it answers                                  |
| -------------------- | ------------------------------------------------ |
| `GET /healthz`       | The environment health of the machine            |
| `GET /readyz`        | Whether the daemon can do work                   |
| `GET /metrics`       | The Prometheus exposition format                 |
| `GET /status`        | The state of every domain                        |
| `POST /run/{domain}` | Start a domain, or `all`                         |
| `POST /cancel`       | Stop the run in progress                         |

The decision record is in
[docs/adr/0001-rust-orchestrator-and-axum.md](docs/adr/0001-rust-orchestrator-and-axum.md).

## Related Repositories

| Repository                                                                | Purpose                                               |
| ------------------------------------------------------------------------- | ----------------------------------------------------- |
| [`foodshare-app`](https://github.com/Foodshareclub/foodshare-app)         | Unified cross-platform app (Skip Fuse: iOS + Android) |
| [`foodshare-web`](https://github.com/Foodshareclub/foodshare-web)         | Next.js 16 web app                                    |
| [`foodshare-backend`](https://github.com/Foodshareclub/foodshare-backend) | Self-hosted Supabase backend                          |
| [`foodshare-runner`](https://github.com/Foodshareclub/foodshare-runner)   | GitHub Actions self-hosted runner                     |

## Development

```bash
cargo test --workspace              # Run all tests
cargo llvm-cov --workspace          # Coverage report
cargo bench --workspace             # Run benchmarks
cargo fmt --all -- --check          # Check formatting
cargo clippy --workspace --all-targets -- -D warnings  # Lint
```

### Toolchain

The Rust in this repository is the toolchain; Bun runs everything around it —
the package manager, the script runner, the TypeScript executor, the test
runner and the type checker. Type checking is `tsrs`, the Rust port of the
TypeScript 7 compiler, with `tsc` on TypeScript 7.1 kept as a parity gate.

Node.js is not part of it. `bun scripts/check-no-node.ts` fails the build if
it creeps back in. `bunx` is the one exception to `[run] bun = true` — it
resolves a package's own shebang — so every `bunx` call here passes `--bun`.

`crates/web` is a standalone Next.js app with its own lockfile, so it carries
its own `bunfig.toml`. Bun resolves `bunfig.toml` from the working directory
only — it does not walk up to the repository root — so a package without one
runs its `node_modules/.bin/*` binaries through their `#!/usr/bin/env node`
shebang. The command still succeeds, on the wrong runtime. The guard checks
every package that declares scripts for exactly this.

`bun cli.ts node-check` runs the guard in all four repositories of the
workspace, and `bun cli.ts verify` runs it before anything else.

Deno appears only where the Supabase Edge Functions are involved, which
live in the backend rather than here.

## Error Codes

| Code Range | Category      | Example                 |
| ---------- | ------------- | ----------------------- |
| E1xxx      | General       | E1001 Internal error    |
| E2xxx      | IO            | E2001 File not found    |
| E3xxx      | Configuration | E3002 Parse error       |
| E4xxx      | Git           | E4001 Not a git repo    |
| E5xxx      | Process       | E5001 Command not found |
| E6xxx      | Validation    | E6001 Invalid input     |
| E7xxx      | Security      | E7001 Secret detected   |
| E8xxx      | Platform      | E8001 Xcode error       |

## License

MIT — see [LICENSE](LICENSE) for details.
