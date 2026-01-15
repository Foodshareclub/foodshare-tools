# foodshare-ios

iOS development tools for Xcode projects, Swift formatting, and simulator management.

## Features

- Xcode project parsing and analysis
- Swift code formatting (SwiftFormat)
- Swift linting (SwiftLint)
- iOS Simulator management
- Build system integration

## CLI Usage

```bash
# Format Swift code
foodshare-ios format --staged

# Lint code
foodshare-ios lint --strict

# Build project
foodshare-ios build --configuration release

# Manage simulators
foodshare-ios simulator list
foodshare-ios simulator boot --device "iPhone 15 Pro"

# Project analysis
foodshare-ios project status
foodshare-ios project missing
foodshare-ios project broken

# Environment check
foodshare-ios doctor
```

## Library Usage

```rust
use foodshare_ios::{xcode, swift, simulator};

// Parse Xcode project
let project = xcode::parse_project("MyApp.xcodeproj")?;

// Format Swift files
swift::format(&["Sources/**/*.swift"], &config)?;

// List simulators
let sims = simulator::list()?;
```

## Modules

### `xcode`

Xcode project operations.

```rust
use foodshare_ios::xcode;

// Parse project
let project = xcode::parse_project("MyApp.xcodeproj")?;

// Find missing files (on disk but not in project)
let missing = xcode::find_missing_files(&project)?;

// Find broken references (in project but not on disk)
let broken = xcode::find_broken_references(&project)?;

// Sync project with disk
xcode::sync_project(&mut project)?;
```

### `swift`

Swift tooling integration.

```rust
use foodshare_ios::swift;

// Format files
swift::format(&["*.swift"], &config)?;

// Lint files
let violations = swift::lint(&["*.swift"], &config)?;

// Get Swift version
let version = swift::version()?;
```

### `simulator`

iOS Simulator management.

```rust
use foodshare_ios::simulator;

// List available simulators
let sims = simulator::list()?;

// Boot simulator
simulator::boot("iPhone 15 Pro")?;

// Shutdown all simulators
simulator::shutdown_all()?;

// Install app
simulator::install(&sim_id, "MyApp.app")?;
```

### `build`

Build system integration.

```rust
use foodshare_ios::build;

// Build project
build::run(&BuildConfig {
    scheme: "MyApp",
    configuration: "Release",
    destination: "generic/platform=iOS",
})?;

// Clean build
build::clean("MyApp.xcodeproj")?;
```

## Configuration

```toml
# .foodshare-hooks.toml

[ios]
# SwiftFormat config
swift_format_config = ".swiftformat"

# SwiftLint config
swift_lint_config = ".swiftlint.yml"

# Default scheme
scheme = "MyApp"

# Default configuration
configuration = "Debug"
```

## Required Tools

| Tool | Purpose | Install |
|------|---------|---------|
| Xcode | Build system | App Store |
| SwiftFormat | Code formatting | `brew install swiftformat` |
| SwiftLint | Linting | `brew install swiftlint` |

## Health Check

```bash
foodshare-ios doctor --json
```

Output:
```json
{
  "status": "healthy",
  "checks": [
    {"name": "git", "status": "healthy", "version": "2.43.0"},
    {"name": "xcodebuild", "status": "healthy", "version": "15.2"},
    {"name": "swift", "status": "healthy", "version": "5.9.2"},
    {"name": "swiftformat", "status": "healthy"},
    {"name": "swiftlint", "status": "healthy"}
  ]
}
```
