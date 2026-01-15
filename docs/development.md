# Development Guide

## Setup

```bash
# Clone repository
git clone https://github.com/Foodshareclub/foodshare-tools.git
cd foodshare-tools

# Build all crates
cargo build

# Run tests
cargo test --workspace
```

## Project Structure

```
foodshare-tools/
├── crates/           # Library crates
├── bins/             # Binary entry points
├── tests/            # Integration tests
├── docs/             # Documentation
└── .snapshots/       # Snapshot test data
```

## Development Commands

```bash
# Build
cargo build                          # Debug build
cargo build --release                # Release build
cargo build -p foodshare-ios         # Single crate

# Test
cargo test --workspace               # All tests
cargo test -p foodshare-core         # Single crate
cargo test -- --nocapture            # Show output

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Format
cargo fmt --all                      # Format code
cargo fmt --all -- --check           # Check formatting

# Documentation
cargo doc --workspace --no-deps      # Generate docs
cargo doc --open                     # Open in browser
```

## Adding a New Crate

1. Create crate directory:
```bash
mkdir -p crates/my-crate/src
```

2. Add `Cargo.toml`:
```toml
[package]
name = "foodshare-my-crate"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
foodshare-core.workspace = true
```

3. Add to workspace in root `Cargo.toml`:
```toml
[workspace]
members = [
    # ...
    "crates/my-crate",
]
```

4. Create `src/lib.rs`:
```rust
//! My crate description.

pub fn hello() -> &'static str {
    "Hello from my-crate!"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {
        assert_eq!(hello(), "Hello from my-crate!");
    }
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function() {
        assert_eq!(my_function(), expected);
    }
}
```

### Integration Tests

Create `tests/integration_test.rs`:

```rust
use foodshare_core::*;

#[test]
fn test_integration() {
    // Test across crate boundaries
}
```

### Snapshot Tests

Using `insta`:

```rust
use insta::assert_snapshot;

#[test]
fn test_output() {
    let output = generate_output();
    assert_snapshot!(output);
}
```

Update snapshots:
```bash
cargo insta review
```

### Property Tests

Using `proptest`:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_property(input in ".*") {
        // Property should hold for any input
        assert!(validate(&input).is_ok() || validate(&input).is_err());
    }
}
```

### Benchmarks

Using `criterion`:

```rust
// benches/my_bench.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark(c: &mut Criterion) {
    c.bench_function("my_function", |b| {
        b.iter(|| my_function())
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
```

Run benchmarks:
```bash
cargo bench --workspace
```

## Code Style

- Follow Rust API guidelines
- Use `rustfmt` defaults
- Document public APIs with `///`
- Use `#[must_use]` for functions returning values
- Prefer `thiserror` for error types

## Error Handling

Use structured error codes:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("[E2001] File not found: {0}")]
    FileNotFound(String),
    
    #[error("[E3001] Invalid configuration: {0}")]
    InvalidConfig(String),
}
```

## Pull Request Checklist

- [ ] Tests pass: `cargo test --workspace`
- [ ] Clippy clean: `cargo clippy --workspace -- -D warnings`
- [ ] Formatted: `cargo fmt --all`
- [ ] Documentation updated
- [ ] CHANGELOG.md updated (if applicable)
- [ ] Commit messages follow conventional commits

## Release Process

See [PUBLISHING.md](../PUBLISHING.md) for release instructions.

## Debugging

### Verbose Output

```bash
RUST_LOG=debug cargo run -p foodshare-ios -- doctor
```

### Backtrace

```bash
RUST_BACKTRACE=1 cargo test
```

### LLDB

```bash
rust-lldb target/debug/foodshare-ios
```
