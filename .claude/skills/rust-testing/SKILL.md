---
name: rust-testing
description: Testing patterns for foodshare-tools Rust workspace. Use when writing tests with insta (snapshots), proptest (property-based), assert_cmd (CLI), or criterion (benchmarks). Covers workspace-wide testing conventions.
---

<objective>
Write comprehensive Rust tests using the workspace's testing tools: insta for snapshots, proptest for property-based testing, assert_cmd for CLI integration, and criterion for benchmarks.
</objective>

<essential_principles>
## Test Commands

```bash
# Run all workspace tests
cargo test --workspace

# Run specific crate tests
cargo test -p foodshare-core

# Run with coverage
cargo llvm-cov --workspace

# Run benchmarks
cargo bench --workspace

# Check all targets (including tests)
cargo check --workspace --all-targets
```

## Unit Tests

Standard Rust test pattern within each module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_commit_message() {
        let config = CommitMsgConfig::default();
        let result = validate("feat: add new feature", &config);
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_invalid_commit_type() {
        let config = CommitMsgConfig::default();
        let result = validate("invalid: bad type", &config);
        assert!(result.is_err());
    }
}
```

## Snapshot Tests (insta)

For testing complex output like formatted strings, error messages, or CLI output:

```rust
use insta::assert_snapshot;

#[test]
fn formats_error_report() {
    let report = generate_report(&findings);
    assert_snapshot!(report);
}

// First run creates snapshot file in snapshots/ directory
// Review with: cargo insta review
```

```bash
# Review new/changed snapshots
cargo insta review

# Update all snapshots
cargo insta accept
```

## Property-Based Tests (proptest)

For testing invariants across random inputs:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn sanitize_never_produces_html(input in ".*") {
        let sanitized = sanitize_input(&input);
        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
    }

    #[test]
    fn commit_msg_length_respected(
        msg in "[a-z]{1,100}: [a-zA-Z0-9 ]{1,200}"
    ) {
        let config = CommitMsgConfig { max_subject_length: 72, ..Default::default() };
        if msg.len() <= 72 {
            assert!(validate(&msg, &config).is_ok());
        }
    }
}
```

## CLI Integration Tests (assert_cmd)

For testing binary crate CLI behavior:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_prints_version() {
    Command::cargo_bin("foodshare-ios")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("foodshare-ios"));
}

#[test]
fn cli_fails_on_invalid_commit() {
    Command::cargo_bin("foodshare-ios")
        .unwrap()
        .args(["commit-msg", "--message", "bad message"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("E6001"));
}
```

## Benchmarks (criterion)

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_secret_scan(c: &mut Criterion) {
    let scanner = SecretScanner::new();
    let files = load_test_files();

    c.bench_function("secret_scan_100_files", |b| {
        b.iter(|| scanner.scan(black_box(&files)))
    });
}

criterion_group!(benches, bench_secret_scan);
criterion_main!(benches);
```

## Test Organization

```
crates/{crate}/
├── src/
│   ├── lib.rs          # Unit tests at bottom of modules
│   └── scanner.rs      # #[cfg(test)] mod tests { ... }
├── tests/              # Integration tests
│   └── integration.rs
├── benches/            # Benchmarks
│   └── benchmark.rs
└── snapshots/          # insta snapshot files
```
</essential_principles>

<success_criteria>
Testing is correct when:
- [ ] All public functions have unit tests
- [ ] Complex output tested with insta snapshots
- [ ] Input validation tested with proptest
- [ ] CLI binaries tested with assert_cmd
- [ ] Performance-critical code benchmarked with criterion
- [ ] `cargo test --workspace` passes cleanly
- [ ] `cargo clippy --all-targets` clean
</success_criteria>
