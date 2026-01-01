# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-12-31

### Added

- Initial release of unified Foodshare Tools
- Core crate with shared functionality:
  - Git operations (command-line based)
  - File scanning utilities
  - Process execution helpers
  - Configuration loading
- Hooks crate with git hook implementations:
  - Conventional commit validation
  - Secret scanning
  - Migration status checking
  - Pre-push validation
- CLI crate with terminal utilities:
  - Status messages
  - Progress indicators
  - Output formatting
- iOS crate with platform-specific tools:
  - Xcode project utilities
  - Simulator management
  - Swift tooling wrappers
- Android crate with platform-specific tools:
  - Gradle integration
  - Emulator management
  - Kotlin tooling wrappers
  - Swift cross-compilation for Android
  - FoodshareCore build scripts
- Web crate with Next.js/React tools:
  - OWASP security scanning
  - Bundle size analysis
  - Accessibility checks
- Three CLI binaries:
  - `foodshare-ios` - iOS development tools
  - `foodshare-android` - Android development tools
  - `lefthook-rs` - Web development hooks

### Migration from Previous Tools

This release consolidates three separate tools directories:
- `foodshare/tools` (lefthook-rs)
- `foodshare-android/tools` (foodshare-hooks)
- `foodshare-ios/tools` (foodshare-hooks)

The shared code has been deduplicated into the core, hooks, and cli crates.
