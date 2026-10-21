# Architecture

## Overview

Foodshare Tools is a Rust monorepo using Cargo workspaces. The architecture follows a layered approach with shared core functionality and platform-specific extensions.

```
foodshare-tools/
├── crates/           # Library crates
│   ├── core/         # Shared infrastructure
│   ├── hooks/        # Git hook implementations
│   ├── cli/          # CLI utilities
│   ├── app/          # Cross-platform app support (iOS + Android)
│   ├── web/          # Web-specific functionality
│   ├── telemetry/    # Observability
│   ├── orchestrator/ # Workspace build and verification orchestration
│   ├── maestro/      # Maestro E2E flows, devices, matrix
│   ├── wasm/         # WebAssembly builds for the published crates
│   ├── xccloud/      # Xcode Cloud over the App Store Connect API
│   ├── device/       # iOS simulators and Android emulators
│   └── [libs]/       # Published libraries (geo, crypto, search, etc.)
├── bins/             # Binary entry points
│   ├── fs-app/         # Cross-platform CLI
│   ├── lefthook-rs/
│   ├── fs-image/
│   ├── fs-auto/        # Headless orchestrator
│   ├── fs-tui/         # Live monitor
│   ├── fs-orchestrator/# HTTP daemon
│   ├── fs-maestro/     # Maestro E2E runner
│   ├── fs-wasm/        # WebAssembly build engine
│   ├── fs-build/       # Cross-platform build engine
│   └── fs-sim/         # Simulator runner
├── tests/            # Integration tests
└── docs/             # Documentation
```

## Dependency Graph

```
                       bins/*
                         │
     ┌───────────────────┼───────────────────┐
     │                   │                   │
  fs-app             fs-auto             fs-tui          fs-orchestrator
     │                   │                   │                   │
     └───────────────────┴─────────┬─────────┴───────────────────┘
                                   │
                          foodshare-orchestrator
                                   │
                       foodshare-core  +  foodshare-telemetry
```

The orchestrator is a library with no HTTP dependency. The three
front-ends — the headless runner, the TUI and the HTTP daemon — are
siblings over it, so the CLI and the TUI do not pull in a web framework
they never use. The daemon is the only crate that depends on axum.

### Core (`foodshare-core`)

Shared infrastructure used by all platform crates:

- **Git operations** - Repository detection, staged files, commit info
- **File scanning** - Pattern matching, directory traversal
- **Process execution** - Command running, output capture
- **Health checks** - Tool availability verification
- **Error types** - Structured error codes (E1xxx-E8xxx)

### Hooks (`foodshare-hooks`)

Git hook implementations:

- **commit-msg** - Conventional commit validation
- **secrets** - Secret/credential scanning (15+ patterns)
- **migrations** - Supabase migration validation
- **pre-push** - Aggregated pre-push checks

### CLI (`foodshare-cli`)

Terminal output utilities:

- **Colored output** - Cross-platform ANSI colors
- **Progress bars** - Indicatif-based progress
- **Tables** - Formatted table output
- **Spinners** - Activity indicators

### Platform Crates

#### iOS (`fs-app`)

- Xcode project parsing (xcodeproj)
- Swift formatting/linting
- Simulator management
- Build system integration

#### Android (`fs-app`)

- Gradle integration
- Kotlin formatting/linting
- Emulator management
- Swift-Android bridge

#### Web (`foodshare-web`)

- Next.js security scanning
- Bundle size analysis
- OWASP checks

### Telemetry (`foodshare-telemetry`)

Observability infrastructure:

- Structured logging (tracing)
- Metrics collection, backed by a Prometheus registry
- Prometheus text exposition
- JSON export for the status endpoint

### Orchestrator (`foodshare-orchestrator`)

Workspace build and verification orchestration:

- The five verification domains: Rust, WASM, Web, Backend, Mobile
- The plan that runs them, with parallel groups and a retry policy
- Disk health and auto-healing between retries
- The state of the run, which the TUI renders and the status endpoint serves
- The full cross-repository verification: the toolchain policy in every
  repository, the disk, the Rust workspace, the web build, the mobile
  smoke tests and the backend edge function tests

The crate has no HTTP dependency. The daemon, the TUI and the headless
runner are three front-ends over it.

### Maestro (`foodshare-maestro`)

Maestro E2E support:

- Device discovery, for iOS simulators and Android emulators
- Flow discovery, in a suite and its subflows
- Syntax checking
- Single-device suites and the multi-device matrix

### Devices (`foodshare-device`)

The iOS simulators and the Android emulators, from the tools that own
them. The app tools and the end-to-end runner share it, so neither of
them parses `simctl` or `adb` output.

### Xcode Cloud (`foodshare-xccloud`)

The App Store Connect integration:

- Credential resolution, from the environment or a `.p8` key file
- The ES256 token the API authenticates with
- The CI products, build runs, actions, issues and artefacts
- The local build summaries CI leaves behind

### WASM (`foodshare-wasm`)

WebAssembly builds for the published crates:

- The five crates that ship to the browser
- The `wasm-pack` invocation
- The loader patch, so Next.js can find the `.wasm` files after bundling

### Published Libraries

Standalone libraries with WASM support:

| Crate                   | Purpose                    |
| ----------------------- | -------------------------- |
| `foodshare-geo`         | Geospatial calculations    |
| `foodshare-crypto`      | HMAC, webhook verification |
| `foodshare-search`      | Fuzzy text search          |
| `foodshare-compression` | Brotli/Gzip                |
| `foodshare-image`       | Image format detection     |

## Error Handling

All errors use structured codes for programmatic handling:

| Range | Category      | Example                 |
| ----- | ------------- | ----------------------- |
| E1xxx | General       | E1001 Internal error    |
| E2xxx | IO            | E2001 File not found    |
| E3xxx | Configuration | E3002 Parse error       |
| E4xxx | Git           | E4001 Not a git repo    |
| E5xxx | Process       | E5001 Command not found |
| E6xxx | Validation    | E6001 Invalid input     |
| E7xxx | Security      | E7001 Secret detected   |
| E8xxx | Platform      | E8001 Xcode error       |

## Configuration

TOML-based configuration with validation:

```toml
# .foodshare-hooks.toml
[commit_msg]
types = ["feat", "fix", "docs", ...]
max_length = 72

[secrets]
exclude_files = ["*.test.ts"]
exclude_patterns = ["EXAMPLE_"]
```

## Performance Considerations

- **Parallel processing** - Rayon for batch operations
- **Lazy evaluation** - Regex compilation with `once_cell`
- **Minimal allocations** - Reuse buffers where possible
- **LTO enabled** - Link-time optimization in release builds

## Testing Strategy

- **Unit tests** - Per-crate in `src/` modules
- **Integration tests** - In `tests/` directory
- **Snapshot tests** - Using `insta` for output verification
- **Property tests** - Using `proptest` for edge cases
- **Benchmarks** - Using `criterion` for performance
