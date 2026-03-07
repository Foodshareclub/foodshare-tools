---
name: crate-development
description: Adding new crates to the foodshare-tools workspace. Use when scaffolding new library or binary crates, setting up Cargo.toml, registering workspace members, and following trait patterns.
disable-model-invocation: true
---

<objective>
Scaffold new crates correctly within the foodshare-tools workspace, following established conventions for Cargo.toml setup, workspace registration, and code patterns.
</objective>

<essential_principles>
## Adding a New Library Crate

### 1. Create crate directory

```bash
mkdir -p crates/{crate-name}/src
```

### 2. Create Cargo.toml

```toml
[package]
name = "foodshare-{crate-name}"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
rust-version.workspace = true
description = "Brief description of the crate"

[dependencies]
# Use workspace dependencies where available
clap = { workspace = true }
thiserror = { workspace = true }

# Internal dependencies
foodshare-core = { path = "../core" }
```

### 3. Register in workspace

Add to root `Cargo.toml`:
```toml
[workspace]
members = [
    # ... existing members
    "crates/{crate-name}",
]
```

### 4. Create src/lib.rs

```rust
//! foodshare-{crate-name}: Brief description
//!
//! This crate provides ...

mod error;

pub use error::Error;
```

## Adding a New Binary Crate

### 1. Create binary directory

```bash
mkdir -p bins/{binary-name}/src
```

### 2. Create Cargo.toml

```toml
[package]
name = "{binary-name}"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
rust-version.workspace = true
description = "Brief description of the binary"

[[bin]]
name = "{binary-name}"
path = "src/main.rs"

[dependencies]
clap = { workspace = true }
foodshare-core = { path = "../../crates/core" }
foodshare-{crate} = { path = "../../crates/{crate}" }
```

### 3. Create src/main.rs

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "{binary-name}", about = "Brief description")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Description of command
    SomeCommand {
        /// Description of arg
        #[arg(short, long)]
        flag: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::SomeCommand { flag } => {
            // implementation
        }
    }
    Ok(())
}
```

## Error Pattern

Every crate should define its own error type:

```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("E{code}: {message}")]
    Structured {
        code: u16,
        message: String,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

## Trait Pattern

Expose functionality through traits for testability:

```rust
pub trait Scanner: Send + Sync {
    fn scan(&self, path: &Path) -> Result<Vec<Finding>>;
}

pub struct SecretScanner { /* ... */ }

impl Scanner for SecretScanner {
    fn scan(&self, path: &Path) -> Result<Vec<Finding>> {
        // implementation
    }
}
```

## Verification

```bash
# Verify workspace builds
cargo check --workspace --all-targets

# Run new crate's tests
cargo test -p foodshare-{crate-name}

# Lint
cargo clippy -p foodshare-{crate-name} -- -D warnings
```
</essential_principles>

<success_criteria>
New crate is correct when:
- [ ] Registered in workspace.members
- [ ] Uses workspace.dependencies for shared deps
- [ ] Has error type with structured error codes
- [ ] Has lib.rs (library) or main.rs (binary)
- [ ] Builds with `cargo check --workspace`
- [ ] Tests pass with `cargo test -p {crate}`
- [ ] Clippy clean
</success_criteria>
