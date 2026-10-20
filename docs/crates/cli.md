# foodshare-cli

CLI utilities for terminal output, progress bars, and user interaction.

## Features

- Colored terminal output
- Progress bars and spinners
- Table formatting
- Cross-platform support

## Usage

```rust
use foodshare_cli::{output, progress, table};

// Colored output
output::success("Build completed");
output::error("Build failed");
output::warning("Deprecated API");

// Progress bar
let pb = progress::bar(100);
for i in 0..100 {
    pb.inc(1);
}
pb.finish();

// Table output
let table = table::new(&["Name", "Status"]);
table.add_row(&["Build", "✓"]);
table.print();
```

## Modules

### `output`

Colored terminal output.

```rust
use foodshare_cli::output;

output::success("Operation completed");
output::error("Operation failed");
output::warning("Deprecation warning");
output::info("Processing files...");
output::debug("Debug info");
```

### `progress`

Progress indicators.

```rust
use foodshare_cli::progress;

// Progress bar
let pb = progress::bar(total);
pb.inc(1);
pb.finish_with_message("Done");

// Spinner
let spinner = progress::spinner("Loading...");
spinner.finish_with_message("Loaded");
```

### `table`

Table formatting.

```rust
use foodshare_cli::table;

let mut t = table::new(&["Check", "Status", "Time"]);
t.add_row(&["Secrets", "✓", "15ms"]);
t.add_row(&["Commit", "✓", "1ms"]);
t.print();
```

## Color Support

Colors are automatically disabled when:
- Output is not a TTY
- `NO_COLOR` environment variable is set
- `--no-color` flag is passed
