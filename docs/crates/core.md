# foodshare-core

Shared infrastructure crate providing foundational functionality for all platform tools.

## Features

- Git operations (repository detection, staged files, commit info)
- File scanning with pattern matching
- Process execution and output capture
- Health check framework
- Structured error types

## Usage

```rust
use foodshare_core::{git, fs, process, health};

// Git operations
let repo = git::find_repo()?;
let staged = git::staged_files(&repo)?;

// File scanning
let matches = fs::scan_directory("src", &["*.rs"])?;

// Process execution
let output = process::run("swift", &["--version"])?;

// Health checks
let status = health::check_tool("swiftformat")?;
```

## Modules

### `git`

Git repository operations.

```rust
use foodshare_core::git;

// Find repository root
let repo_root = git::find_repo()?;

// Get staged files
let staged = git::staged_files(&repo_root)?;

// Get commit message
let msg = git::read_commit_msg(".git/COMMIT_MSG")?;

// Check if path is ignored
let ignored = git::is_ignored(&repo_root, "node_modules")?;
```

### `fs`

File system operations.

```rust
use foodshare_core::fs;

// Scan directory with patterns
let files = fs::scan_directory("src", &["*.rs", "*.toml"])?;

// Read file content
let content = fs::read_file("config.toml")?;

// Check file exists
let exists = fs::exists("Cargo.toml");
```

### `process`

Process execution.

```rust
use foodshare_core::process;

// Run command
let output = process::run("swift", &["--version"])?;

// Run with timeout
let output = process::run_with_timeout("long-command", &[], Duration::from_secs(30))?;

// Check command exists
let exists = process::command_exists("swiftformat");
```

### `health`

Health check framework.

```rust
use foodshare_core::health::{HealthCheck, HealthStatus};

// Check single tool
let check = HealthCheck::new("swiftformat")
    .with_version_arg("--version")
    .check()?;

// Aggregate checks
let checks = vec![
    HealthCheck::new("git"),
    HealthCheck::new("swift"),
    HealthCheck::new("swiftformat"),
];

let results: Vec<HealthStatus> = checks
    .iter()
    .map(|c| c.check())
    .collect();
```

### `error`

Structured error types.

```rust
use foodshare_core::error::{Error, ErrorCode};

// Create error with code
let err = Error::new(ErrorCode::FileNotFound, "config.toml not found");

// Error codes
// E1xxx - General
// E2xxx - IO
// E3xxx - Configuration
// E4xxx - Git
// E5xxx - Process
// E6xxx - Validation
// E7xxx - Security
// E8xxx - Platform
```

## Error Codes

| Code | Description |
|------|-------------|
| E1001 | Internal error |
| E2001 | File not found |
| E2002 | Permission denied |
| E3001 | Config not found |
| E3002 | Config parse error |
| E4001 | Not a git repository |
| E4002 | Git command failed |
| E5001 | Command not found |
| E5002 | Command failed |
| E6001 | Invalid input |
| E7001 | Secret detected |
