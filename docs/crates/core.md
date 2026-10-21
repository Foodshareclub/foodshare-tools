# foodshare-core

Shared infrastructure crate providing foundational functionality for all platform tools.

## Features

- Git operations (repository detection, staged files, commit info)
- File scanning with pattern matching
- Process execution, with an optional deadline
- Health check framework
- Structured error types, with codes and exit codes
- The shared configuration schema, and the report of the keys it does not read

## Usage

```rust
use foodshare_core::{git, process};
use std::time::Duration;

// Git operations
let repo = git::GitRepo::open_current()?;
let staged = repo.staged_files()?;

// File scanning
let files = foodshare_core::file_scanner::scan_swift_files(std::path::Path::new("."))?;

// Process execution
let output = process::run_command("swift", &["--version"])?;

// A command that may hang, given a deadline
let output = process::run_command_with_timeout("xcodebuild", &[], Duration::from_secs(600))?;
```

## Modules

### `git`

Git repository operations. The repository is a value you open and then
ask questions of; there are no free functions that go looking for it.

```rust
use foodshare_core::git::GitRepo;

// Open the repository the working directory is in
let repo = GitRepo::open_current()?;

// The files that are staged
let staged = repo.staged_files()?;

// The staged files of one kind
let swift = repo.staged_swift_files()?;

// The branch
let branch = repo.current_branch()?;

// Whether a path is ignored
let ignored = repo.is_ignored(std::path::Path::new("node_modules"));
```

### `file_scanner`

File system scanning.

```rust
use foodshare_core::file_scanner::FileScanner;

// Scan for a set of extensions, with exclusions
let files = FileScanner::new("src")
    .with_extensions(&["ts", "tsx"])
    .exclude(&["**/node_modules/**"])
    .scan()?;

// Or one of the shortcuts
let swift = foodshare_core::file_scanner::scan_swift_files(std::path::Path::new("."))?;
```

### `process`

Process execution.

```rust
use foodshare_core::process;
use std::time::Duration;

// Run a command
let output = process::run_command("swift", &["--version"])?;

// Run a command in a directory
let output = process::run_command_in_dir("swiftlint", &["lint"], std::path::Path::new("src"))?;

// Run a command that may hang, with a deadline. The child is killed when
// the deadline passes, and the error says so.
let output = process::run_command_with_timeout("xcodebuild", &[], Duration::from_secs(600))?;

// Check a command exists
let exists = process::command_exists("swiftformat");
```

### `health`

Health check framework.

```rust
use foodshare_core::health::{CommandCheck, HealthChecker};

// Aggregate the standard checks
let report = HealthChecker::new().with_standard_checks().run();

// Or build the checks yourself
let report = HealthChecker::new()
    .add_check(CommandCheck::new("swiftformat", Some("--version")))
    .run();
```

### `config`

The shared configuration schema, and the report of the keys this build
does not implement.

```rust
use foodshare_core::config::Config;

let config = Config::load(None)?;

// The keys in the file that this build does not act on
for key in &config.unknown_keys {
    eprintln!("{key} is not implemented by this tool");
}
```

### `error`

Structured error types. The code decides the exit code, so a caller can
tell a secret that leaked from a message that was too long.

```rust
use foodshare_core::error::{Error, ErrorCode};

let err = Error::new(ErrorCode::SecretDetected, "a token was found");

assert_eq!(err.exit_code(), 5);
assert_eq!(err.coded_message(), "E7001: a token was found");
```

## Error Codes

| Code | Description | Exit |
|------|-------------|------|
| E1001 | Internal error | 1 |
| E2001 | File not found | 1 |
| E2002 | Permission denied | 1 |
| E3001 | Config not found | 3 |
| E3002 | Config parse error | 3 |
| E4001 | Not a git repository | 4 |
| E4002 | Git command failed | 4 |
| E5001 | Command not found | 127 |
| E5002 | Command failed | 1 |
| E5003 | Process timed out | 124 |
| E6001 | Invalid input | 2 |
| E7001 | Secret detected | 5 |
