# Contributing to Foodshare Tools

Thank you for your interest in contributing to Foodshare Tools! This document provides guidelines for contributing.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/foodshare-tools.git`
3. Create a branch: `git checkout -b feature/your-feature`
4. Make your changes
5. Run tests: `cargo test --workspace`
6. Commit with conventional commits: `git commit -m "feat: add new feature"`
7. Push and create a Pull Request

## Development Setup

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- For iOS development: Xcode, swiftformat, swiftlint
- For Android development: Android SDK, ktlint, detekt
- For web development: Node.js

### Building

```bash
# Build all crates
cargo build --workspace

# Build specific binary
cargo build -p foodshare-ios-cli
cargo build -p foodshare-android-cli
cargo build -p lefthook-rs

# Run tests
cargo test --workspace

# Run clippy
cargo clippy --workspace -- -D warnings

# Format code
cargo fmt --all
```

## Code Style

- Follow Rust conventions and idioms
- Use `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Write tests for new functionality
- Document public APIs with doc comments

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `style:` - Code style changes (formatting, etc.)
- `refactor:` - Code refactoring
- `test:` - Adding or updating tests
- `chore:` - Maintenance tasks

## Pull Request Process

1. Update documentation if needed
2. Add tests for new functionality
3. Ensure all tests pass
4. Update CHANGELOG.md if applicable
5. Request review from maintainers

## Reporting Issues

- Use GitHub Issues
- Include reproduction steps
- Include Rust version (`rustc --version`)
- Include OS and platform information

## Code of Conduct

Be respectful and inclusive. We follow the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
