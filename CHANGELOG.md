# Changelog

All notable changes to foodshare-tools will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security
- Fixed critical shell command injection vulnerability in `command_exists()` and `which_command()`
  functions by replacing shell-based lookups with the `which` crate

### Added
- Comprehensive test suite for secrets detection (45 tests covering all patterns)
- `#![warn(missing_docs)]` to cli, hooks, ios, android, web, image, search, compression, and crypto crates

### Changed
- Rate limiter now handles poisoned locks gracefully instead of panicking
- Improved file name handling in fs-image to avoid panics on edge cases

### Removed
- Unused `petgraph` dependency from ios and android crates

## [1.4.0] - 2025-01-15

### Added
- foodshare-api-client crate with circuit breaker and rate limiting
- Image alpha channel removal for App Store screenshots
- Smart image resize recommendations based on file size tiers
- Delta sync support for translations

### Changed
- Migrated to Rust 2024 edition
- Updated to Rust 1.85 minimum version
- Improved error messages across all CLI tools

### Fixed
- Race condition in telemetry batch flushing
- Memory leak in image processing pipeline

## [1.3.0] - 2024-12-01

### Added
- Code protection system for iOS
- Swift toolchain management
- Enterprise git hooks

### Changed
- Restructured crates for better modularity
- Improved build times with incremental compilation hints

## [1.2.0] - 2024-10-15

### Added
- Android platform support
- Web platform support (Next.js tools)
- Brotli compression support

### Changed
- Unified CLI output formatting

## [1.1.0] - 2024-08-01

### Added
- Secret scanning with 11 pattern types
- Migration validation hooks
- Conventional commit enforcement

## [1.0.0] - 2024-06-01

### Added
- Initial release
- Core utilities (config, error handling, process execution)
- iOS tools (Xcode, simulator, Swift)
- Git hooks framework
- Telemetry and metrics

[Unreleased]: https://github.com/Foodshareclub/foodshare-tools/compare/v1.4.0...HEAD
[1.4.0]: https://github.com/Foodshareclub/foodshare-tools/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/Foodshareclub/foodshare-tools/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/Foodshareclub/foodshare-tools/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/Foodshareclub/foodshare-tools/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/Foodshareclub/foodshare-tools/releases/tag/v1.0.0
