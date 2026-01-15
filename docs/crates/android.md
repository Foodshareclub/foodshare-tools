# foodshare-android

Android development tools for Gradle projects, Kotlin formatting, and emulator management.

## Features

- Gradle build integration
- Kotlin code formatting
- Android emulator management
- Swift-Android bridge support

## CLI Usage

```bash
# Format Kotlin code
foodshare-android format --lang kotlin

# Lint code
foodshare-android lint

# Build Swift core for Android
foodshare-android swift-core build --target arm64
foodshare-android swift-core copy --output app/libs

# Manage emulators
foodshare-android emulator list
foodshare-android emulator boot pixel_7

# Environment check
foodshare-android doctor
```

## Library Usage

```rust
use foodshare_android::{gradle, kotlin, emulator, swift_core};

// Run Gradle task
gradle::run("assembleDebug")?;

// Format Kotlin files
kotlin::format(&["**/*.kt"])?;

// List emulators
let emus = emulator::list()?;

// Build Swift core
swift_core::build("arm64")?;
```

## Modules

### `gradle`

Gradle build integration.

```rust
use foodshare_android::gradle;

// Run task
gradle::run("assembleDebug")?;

// Run with arguments
gradle::run_with_args("build", &["--info"])?;

// Clean
gradle::clean()?;
```

### `kotlin`

Kotlin tooling integration.

```rust
use foodshare_android::kotlin;

// Format files
kotlin::format(&["**/*.kt"])?;

// Lint files
let issues = kotlin::lint(&["**/*.kt"])?;
```

### `emulator`

Android emulator management.

```rust
use foodshare_android::emulator;

// List available emulators
let emus = emulator::list()?;

// Boot emulator
emulator::boot("pixel_7")?;

// Shutdown emulator
emulator::shutdown("pixel_7")?;

// Install APK
emulator::install(&emu_id, "app-debug.apk")?;
```

### `swift_core`

Swift-Android bridge.

```rust
use foodshare_android::swift_core;

// Build for target
swift_core::build("arm64-v8a")?;

// Copy libraries
swift_core::copy_libs("app/libs")?;

// Supported targets
let targets = swift_core::supported_targets();
// ["arm64-v8a", "armeabi-v7a", "x86_64", "x86"]
```

## Configuration

```toml
# .foodshare-hooks.toml

[android]
# Kotlin format config
kotlin_config = ".editorconfig"

# Default build variant
build_variant = "debug"

# Swift core output directory
swift_core_output = "app/libs"
```

## Required Tools

| Tool | Purpose | Install |
|------|---------|---------|
| Android SDK | Build system | Android Studio |
| Gradle | Build tool | Bundled with project |
| ktlint | Kotlin linting | `brew install ktlint` |

## Health Check

```bash
foodshare-android doctor --json
```
