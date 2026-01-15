# Foodshare Tools Documentation

Welcome to the Foodshare Tools documentation. This monorepo contains enterprise-grade Rust CLI tools and libraries for the Foodshare platform.

## Quick Links

| Document | Description |
|----------|-------------|
| [Getting Started](./getting-started.md) | Installation and first steps |
| [Architecture](./architecture.md) | System design and crate structure |
| [CLI Reference](./cli-reference.md) | Command-line interface documentation |
| [Configuration](./configuration.md) | TOML configuration options |
| [Development](./development.md) | Contributing and local development |

## Crate Documentation

### Platform CLIs

| Crate | Description |
|-------|-------------|
| [foodshare-ios](./crates/ios.md) | iOS development tools (Xcode, Swift, simulators) |
| [foodshare-android](./crates/android.md) | Android development tools (Gradle, Kotlin, emulators) |
| [foodshare-web](./crates/web.md) | Web development tools (Next.js security, bundle analysis) |

### Core Libraries

| Crate | Description |
|-------|-------------|
| [foodshare-core](./crates/core.md) | Shared infrastructure (git, file scanning, process) |
| [foodshare-hooks](./crates/hooks.md) | Git hooks (commit-msg, secrets, migrations) |
| [foodshare-cli](./crates/cli.md) | CLI utilities (terminal output, progress bars) |
| [foodshare-telemetry](./crates/telemetry.md) | Observability (logging, metrics, tracing) |

### Published Libraries

| Crate | Description |
|-------|-------------|
| [foodshare-geo](./crates/geo.md) | Geospatial utilities |
| [foodshare-crypto](./crates/crypto.md) | Cryptographic utilities |
| [foodshare-search](./crates/search.md) | Fuzzy search |
| [foodshare-compression](./crates/compression.md) | Brotli/Gzip compression |
| [foodshare-image](./crates/image.md) | Image format detection |

## Binaries

| Binary | Description |
|--------|-------------|
| `foodshare-ios` | iOS CLI |
| `foodshare-android` | Android CLI |
| `lefthook-rs` | Web CLI |
| `fs-image` | Image processing CLI |
| `motherduck-sync` | Database sync CLI |
