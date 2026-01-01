# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.x.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please report it responsibly.

### How to Report

1. **Do NOT** create a public GitHub issue for security vulnerabilities
2. Email security@foodshare.club with:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Any suggested fixes

### What to Expect

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 7 days
- **Resolution Timeline**: Depends on severity
  - Critical: 24-48 hours
  - High: 7 days
  - Medium: 30 days
  - Low: 90 days

### Security Measures

This project implements several security measures:

1. **Secret Scanning**: Built-in detection for 15+ secret patterns
2. **Dependency Auditing**: Regular `cargo audit` checks in CI
3. **Code Review**: All changes require review before merge
4. **Signed Releases**: All releases are signed and checksummed

### Scope

The following are in scope for security reports:

- Code execution vulnerabilities
- Secret/credential exposure
- Path traversal attacks
- Denial of service
- Dependency vulnerabilities

### Out of Scope

- Social engineering attacks
- Physical attacks
- Issues in dependencies (report to upstream)

## Security Best Practices

When using these tools:

1. Keep tools updated to the latest version
2. Review `.foodshare-hooks.toml` configuration
3. Don't disable secret scanning in production
4. Use environment variables for sensitive configuration
5. Regularly audit your git hooks configuration

## Acknowledgments

We appreciate responsible disclosure and will acknowledge security researchers who report valid vulnerabilities.
