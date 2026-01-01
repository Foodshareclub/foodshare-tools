# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.1] - 2026-01-01

### Fixed
- **Documentation**: Added comprehensive documentation to all public APIs
  - Eliminated all 82 compiler warnings for missing documentation
  - Documented all enum variants, struct fields, and trait methods
  - Added doc comments to `ResultExt` trait methods
  - Documented `HealthStatus`, `CircuitState`, `AuditAction`, `AuditSeverity` variants
  - Documented `CacheStats`, `DiffStats`, `CheckResult`, `HealthReport` fields
  - Documented `Config` and `ConfigSchema` fields

### Changed
- Removed unused imports in `health.rs`, `retry.rs`, and `cache.rs`

## [1.3.0] - 2026-01-01

### Added
- **Rate limiting module** (`foodshare_core::rate_limit`) with:
  - Token bucket algorithm for smooth rate limiting
  - Sliding window limiter for accurate request counting
  - Burst allowance for handling traffic spikes
  - Per-key rate limiting for multi-tenant scenarios
  - Preset configurations: `per_second()`, `per_minute()`, `strict()`, `lenient()`
  - Status reporting with available tokens and reset time

- **Feature flags module** (`foodshare_core::feature_flags`) with:
  - Boolean, string, numeric, and percentage-based flags
  - Environment variable overrides
  - Tag-based flag grouping
  - Percentage rollouts for gradual feature releases
  - JSON file loading and hot-reload support
  - Default flags for common tooling settings

### Changed
- **Prelude** expanded to include:
  - `FeatureFlags`, `Flag`, `FlagValue`
  - `RateLimitConfig`, `RateLimiter`

## [1.2.0] - 2026-01-01

### Added
- **Cache module** (`foodshare_core::cache`) with:
  - File-based caching with optional in-memory layer
  - TTL (time-to-live) support with automatic expiry
  - Data integrity verification via SHA-256 hashing
  - Cache statistics and cleanup utilities
  - `cached_command` helper for caching expensive operations

- **Retry module** (`foodshare_core::retry`) with:
  - Exponential backoff with configurable jitter
  - Preset configurations: `quick()`, `patient()`, `no_retry()`
  - Circuit breaker pattern for preventing cascading failures
  - Configurable failure/success thresholds
  - Half-open state for gradual recovery

- **Audit module** (`foodshare_core::audit`) with:
  - Structured audit logging for security and compliance
  - Severity levels (Low, Medium, High, Critical)
  - Action types for commands, config, security, git, and system events
  - Log rotation with configurable file size limits
  - JSON and human-readable output formats
  - Global audit log instance with `audit!` macro

- **Validation module** (`foodshare_core::validation`) with:
  - Fluent builder API for validation rules
  - Built-in validators: required, min/max length, pattern, one_of, range
  - Path validators: exists, is_file, is_directory
  - Custom validation support
  - Commit message format validation
  - Path safety validation (traversal detection)
  - Configuration schema validation

### Changed
- **Prelude** expanded to include new module types:
  - `AuditAction`, `AuditEvent`, `AuditLog`
  - `Cache`, `CacheConfig`
  - `retry`, `CircuitBreaker`, `RetryConfig`
  - `ValidationResult`, `Validator`

### Dependencies
- Added `sha2` for cache integrity hashing
- Added `hex` for hash encoding
- Added `uuid` for audit event IDs
- Added `dirs` for platform-specific cache directories

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

[1.3.1]: https://github.com/Foodshareclub/foodshare-tools/compare/v1.3.0...v1.3.1
[1.3.0]: https://github.com/Foodshareclub/foodshare-tools/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/Foodshareclub/foodshare-tools/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/Foodshareclub/foodshare-tools/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/Foodshareclub/foodshare-tools/releases/tag/v1.0.0
