# Architecture

## Overview

Foodshare Tools is a Rust monorepo using Cargo workspaces. The architecture follows a layered approach with shared core functionality and platform-specific extensions.

```
foodshare-tools/
├── crates/           # Library crates
│   ├── core/         # Shared infrastructure
│   ├── hooks/        # Git hook implementations
│   ├── cli/          # CLI utilities
│   ├── ios/          # iOS-specific functionality
│   ├── android/      # Android-specific functionality
│   ├── web/          # Web-specific functionality
│   ├── telemetry/    # Observability
│   └── [libs]/       # Published libraries (geo, crypto, search, etc.)
├── bins/             # Binary entry points
│   ├── foodshare-ios/
│   ├── foodshare-android/
│   ├── lefthook-rs/
│   └── fs-image/
├── tests/            # Integration tests
└── docs/             # Documentation
```

## Dependency Graph

```
                    ┌─────────────┐
                    │   bins/*    │  Binary entry points
                    └──────┬──────┘
                           │
         ┌─────────────────┼─────────────────┐
         │                 │                 │
    ┌────▼────┐      ┌─────▼─────┐     ┌─────▼─────┐
    │   ios   │      │  android  │     │    web    │  Platform crates
    └────┬────┘      └─────┬─────┘     └─────┬─────┘
         │                 │                 │
         └─────────────────┼─────────────────┘
                           │
                    ┌──────▼──────┐
                    │    hooks    │  Git hook logic
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
         ┌────▼────┐  ┌────▼────┐  ┌────▼────┐
         │   cli   │  │telemetry│  │  core   │  Foundation
         └─────────┘  └─────────┘  └─────────┘
```

## Crate Responsibilities

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

#### iOS (`foodshare-ios`)
- Xcode project parsing (xcodeproj)
- Swift formatting/linting
- Simulator management
- Build system integration

#### Android (`foodshare-android`)
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
- Metrics collection
- Prometheus export

### Published Libraries

Standalone libraries with WASM support:

| Crate | Purpose |
|-------|---------|
| `foodshare-geo` | Geospatial calculations |
| `foodshare-crypto` | HMAC, webhook verification |
| `foodshare-search` | Fuzzy text search |
| `foodshare-compression` | Brotli/Gzip |
| `foodshare-image` | Image format detection |

## Error Handling

All errors use structured codes for programmatic handling:

| Range | Category | Example |
|-------|----------|---------|
| E1xxx | General | E1001 Internal error |
| E2xxx | IO | E2001 File not found |
| E3xxx | Configuration | E3002 Parse error |
| E4xxx | Git | E4001 Not a git repo |
| E5xxx | Process | E5001 Command not found |
| E6xxx | Validation | E6001 Invalid input |
| E7xxx | Security | E7001 Secret detected |
| E8xxx | Platform | E8001 Xcode error |

## Configuration

TOML-based configuration with validation:

```toml
# .foodshare-hooks.toml
[commit_msg]
types = ["feat", "fix", "docs", ...]
max_subject_length = 72

[secrets]
exclude_files = ["*.test.ts"]
exclude_patterns = ["EXAMPLE_"]

[migrations]
directory = "supabase/migrations"
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
