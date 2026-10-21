# foodshare-cli

Terminal output helpers shared by the Foodshare command-line tools.

## Features

- Status messages in one shape, so every tool says the same things the
  same way
- Errors that carry their code, so a caller can tell a security failure
  from a validation failure
- Formatting for durations, sizes and counts

## Usage

```rust
use foodshare_cli::output::Status;

// Colored output
Status::success("Build completed");
Status::error("Build failed");
Status::warning("Deprecated API");
Status::info("Processing files...");

// An error with its code
Status::failure(&error);
```

## Modules

### `output`

Colored terminal output.

```rust
use foodshare_cli::output::Status;

Status::success("Operation completed");
Status::error("Operation failed");
Status::warning("Deprecation warning");
Status::info("Processing files...");
Status::header("A section");
Status::step(1, 3, "The first of three");
```

The formatting helpers are free functions:

```rust
use foodshare_cli::output::{format_count, format_duration, format_size};

assert_eq!(format_duration(std::time::Duration::from_millis(500)), "500ms");
assert_eq!(format_size(2048), "2.00 KB");
assert_eq!(format_count(1, "file", "files"), "1 file");
```
