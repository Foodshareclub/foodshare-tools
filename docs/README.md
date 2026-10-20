# Foodshare Tools Documentation

Welcome to the Foodshare Tools documentation. This monorepo contains enterprise-grade Rust CLI tools and libraries for the Foodshare platform.

## Quick Links

| Document                                | Description                          |
| --------------------------------------- | ------------------------------------ |
| [Getting Started](./getting-started.md) | Installation and first steps         |
| [Architecture](./architecture.md)       | System design and crate structure    |
| [CLI Reference](./cli-reference.md)     | Command-line interface documentation |
| [Configuration](./configuration.md)     | TOML configuration options           |
| [Development](./development.md)         | Contributing and local development   |

## Crate Documentation

### Platform CLIs

| Crate                            | Description                                                                    |
| -------------------------------- | ------------------------------------------------------------------------------ |
| [fs-app](./crates/app.md)        | Cross-platform (iOS, Android) CLI tools (Xcode, Gradle, simulators, emulators) |
| [foodshare-web](./crates/web.md) | Web development tools (Next.js security, bundle analysis)                      |

### Core Libraries

| Crate                                        | Description                                         |
| -------------------------------------------- | --------------------------------------------------- |
| [foodshare-core](./crates/core.md)           | Shared infrastructure (git, file scanning, process) |
| [foodshare-hooks](./crates/hooks.md)         | Git hooks (commit-msg, secrets, migrations)         |
| [foodshare-cli](./crates/cli.md)             | CLI utilities (terminal output, progress bars)      |
| [foodshare-telemetry](./crates/telemetry.md) | Observability (logging, metrics, tracing)           |

### Published Libraries

| Crate                                            | Description             |
| ------------------------------------------------ | ----------------------- |
| [foodshare-geo](./crates/geo.md)                 | Geospatial utilities    |
| [foodshare-crypto](./crates/crypto.md)           | Cryptographic utilities |
| [foodshare-search](./crates/search.md)           | Fuzzy search            |
| [foodshare-compression](./crates/compression.md) | Brotli/Gzip compression |
| [foodshare-image](./crates/image.md)             | Image format detection  |

## Binaries

| Binary              | Description                                                           |
| ------------------- | --------------------------------------------------------------------- |
| `fs-app`            | Cross-platform (iOS, Android) CLI and project management              |
| `lefthook-rs`       | Git hooks CLI (security, conventional commits, branch/file gates)     |
| `fs-image`          | Image processing CLI                                                  |
| `foodshare-i18n`    | Translation management CLI ([docs](../bins/foodshare-i18n/README.md)) |
| `foodshare-swift`   | Swift toolchain version management CLI                                |
| `foodshare-migrate` | Secret migration CLI for self-hosted Supabase                         |
