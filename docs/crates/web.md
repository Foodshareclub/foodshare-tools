# foodshare-web

Web development tools for Next.js security scanning and bundle analysis.

## Features

- OWASP security scanning
- Next.js-specific security checks
- Bundle size analysis
- Conventional commit validation

## CLI Usage

```bash
# Security checks
lefthook-rs security
lefthook-rs nextjs-security src/**/*.tsx

# Bundle size analysis
lefthook-rs bundle-size --threshold 500kb

# Conventional commit validation
lefthook-rs conventional-commit .git/COMMIT_MSG
```

## Security Checks

### OWASP Categories

- A01: Broken Access Control
- A02: Cryptographic Failures
- A03: Injection
- A07: XSS
- A10: SSRF

## Configuration

```toml
[web]
bundle_threshold = 500000
scan_dirs = ["src", "app"]
exclude = ["*.test.ts"]
```
