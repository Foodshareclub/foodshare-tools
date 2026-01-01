# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2026-01-01

### Added
- **Telemetry crate** (`foodshare-telemetry`) with:
  - Structured logging via `tracing`
  - Metrics collection with counters, gauges, and histograms
  - Session tracking with UUIDs
  - Timer utilities for performance measurement
  - JSON export for metrics

- **Health check system** (`foodshare_core::health`) with:
  - `HealthChecker` for running environment checks
  - Built-in checks for git, commands, environment variables, disk space
  - Platform-specific check presets (iOS, Android, Web)
  - JSON-serializable health reports

- **Enhanced error handling** with:
  - Error codes (E1xxx-E8xxx) for programmatic handling
  - Error context and recovery suggestions
  - Serializable error reports
  - Category-based error classification

- **Security policy** (`SECURITY.md`) with:
  - Vulnerability reporting guidelines
  - Response timeline expectations
  - Security best practices

### Changed
- **Error type** changed from enum variants to struct with constructor methods
  - Use `Error::git()`, `Error::config()`, etc. instead of `Error::Git()`
  - Errors now include optional context and suggestions
- **CI/CD improvements**:
  - Added security audit job with `cargo-audit`
  - Added code coverage with `cargo-llvm-cov`
  - Added MSRV (1.75) verification
  - Added concurrency controls to prevent duplicate runs
- **Release profile** optimized with LTO and stripped binaries
- **Documentation** expanded with error codes table and performance benchmarks

### Fixed
- Error handling consistency across all crates

## [1.0.0] - 2025-12-31

### Added
- Initial release of unified Foodshare tools
- **Core crate** (`foodshare-core`) with:
  - Git operations via command-line git
  - File scanning and filtering
  - Process execution utilities
  - TOML configuration loading

- **Hooks crate** (`foodshare-hooks`) with:
  - Conventional commit validation
  - Secret scanning (15+ patterns)
  - Migration checks
  - Pre-push validation

- **CLI crate** (`foodshare-cli`) with:
  - Terminal output utilities
  - Progress indicators
  - Color support

- **iOS crate** (`foodshare-ios`) with:
  - Xcode build and test commands
  - Simulator management
  - Swift formatting and linting
  - Xcode project analysis (xcodeproj)

- **Android crate** (`foodshare-android`) with:
  - Gradle integration
  - Emulator management
  - Kotlin tools
  - Swift-Android cross-compilation

- **Web crate** (`foodshare-web`) with:
  - Next.js security checks
  - Bundle size analysis
  - Accessibility checks

- **Binaries**:
  - `foodshare-ios` - iOS development CLI
  - `foodshare-android` - Android development CLI
  - `lefthook-rs` - Web development CLI

- **Documentation**:
  - README with usage examples
  - CONTRIBUTING guidelines
  - CODE_OF_CONDUCT
  - MIT LICENSE

- **CI/CD**:
  - GitHub Actions for CI
  - Multi-platform builds (Linux, macOS x64/ARM)
  - Automated releases

[1.1.0]: https://github.com/Foodshareclub/foodshare-tools/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/Foodshareclub/foodshare-tools/releases/tag/v1.0.0
