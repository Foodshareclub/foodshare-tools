# Contributing to Foodshare Tools

Thank you for your interest in contributing to Foodshare Tools! This document provides guidelines and instructions for contributing.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for everyone.

## Getting Started

### Prerequisites

- Rust 1.85 or later (Edition 2024)
- macOS, Linux, or Windows

### Setting Up the Development Environment

```bash
# Clone the repository
git clone https://github.com/Foodshareclub/foodshare-tools.git
cd foodshare-tools

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace
```

## Development Workflow

### Branching Strategy

- `main` - stable release branch
- `develop` - integration branch for features
- `feature/*` - feature branches
- `fix/*` - bug fix branches

### Making Changes

1. Create a new branch from `develop`
2. Make your changes
3. Add tests for new functionality
4. Ensure all tests pass: `cargo test --workspace`
5. Run clippy: `cargo clippy --workspace -- -D warnings`
6. Format code: `cargo fmt --all`
7. Commit with conventional commit messages

### Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

Examples:
```
feat(ios): add simulator boot command
fix(hooks): handle edge case in secret detection
docs(core): improve rate limiter documentation
```

## Code Standards

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `#[must_use]` for functions that return values that should be used
- Document all public APIs with doc comments
- Add `# Errors` section for functions returning `Result`
- Add `# Panics` section if the function can panic

### Testing

- Write unit tests for all new functionality
- Use property-based testing with `proptest` for complex logic
- Integration tests go in `tests/` directory
- Aim for meaningful test coverage, not arbitrary percentages

### Documentation

- All public items must have documentation
- Include examples in doc comments where helpful
- Keep README files updated

## Project Structure

```
foodshare-tools/
├── Cargo.toml          # Workspace configuration
├── crates/             # Library crates
│   ├── core/           # Core utilities
│   ├── hooks/          # Git hooks
│   ├── cli/            # CLI utilities
│   ├── ios/            # iOS tools
│   ├── android/        # Android tools
│   ├── web/            # Web tools
│   └── ...
├── bins/               # Binary crates
│   ├── foodshare-ios/  # iOS CLI
│   ├── fs-image/       # Image processing CLI
│   └── ...
└── docs/               # Documentation
```

## Pull Request Process

1. Update CHANGELOG.md with your changes
2. Ensure CI passes
3. Request review from maintainers
4. Address feedback
5. Squash commits if requested
6. Merge after approval

## Security

If you discover a security vulnerability, please do NOT open a public issue. Instead, email security@foodshare.club with details.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

## Questions?

Feel free to open a discussion or reach out to the maintainers.
