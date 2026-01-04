# Contributing to Foodshare Tools

Thank you for your interest in contributing! This document provides guidelines for contributing to our Rust crates and WASM packages.

## Getting Started

### Prerequisites

- Rust 1.75 or later
- wasm-pack (for WASM builds)
- Node.js 18+ (for npm packages)

### Setup

```bash
git clone https://github.com/Foodshareclub/foodshare-tools.git
cd foodshare-tools
cargo build
cargo test
```

## Development Workflow

### 1. Fork and Clone

```bash
git clone https://github.com/YOUR_USERNAME/foodshare-tools.git
git remote add upstream https://github.com/Foodshareclub/foodshare-tools.git
```

### 2. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/your-bug-fix
```

### 3. Make Changes

- Follow existing code style
- Add tests for new functionality
- Ensure all tests pass: `cargo test`

### 4. Commit Guidelines

We follow conventional commits:

```
feat: add new distance calculation method
fix: correct PostGIS WKT parsing edge case
docs: update README with usage examples
perf: optimize batch distance calculation
```

### 5. Submit Pull Request

1. Push to your fork
2. Open a Pull Request against `main`
3. Wait for review

## Testing

```bash
cargo test                           # All tests
cargo test -p foodshare-geo          # Specific crate
cargo bench -p foodshare-geo         # Benchmarks
```

## Building WASM

```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
wasm-pack build crates/geo --target web --features wasm
```

## License

By contributing, you agree that your contributions will be licensed under MIT.
