# CLI Reference

## Common Commands

These commands are available across all platform binaries (`fs-app`, `fs-app`, `lefthook-rs`).

### commit-msg

Validate commit message format (conventional commits).

```bash
<binary> commit-msg <file>
<binary> commit-msg -        # Read from stdin
```

Options:
- `--types <list>` - Allowed commit types (default: feat,fix,docs,style,refactor,test,chore,ci,perf)
- `--max-length <n>` - Maximum subject length (default: 72)
- `--require-scope` - Require scope in commit message

### secrets

Scan for secrets and credentials in staged files.

```bash
<binary> secrets
<binary> secrets --all       # Scan all files, not just staged
```

Options:
- `--exclude <pattern>` - Exclude files matching pattern
- `--json` - Output results as JSON
- `--fail-on-warning` - Exit non-zero on warnings

Detected patterns:
- AWS keys, secrets, session tokens
- Supabase keys (anon, service role)
- GitHub tokens
- Stripe keys
- Generic API keys and passwords

### migrations

Validate Supabase migrations.

```bash
<binary> migrations
<binary> migrations --dir supabase/migrations
```

Options:
- `--dir <path>` - Migrations directory (default: supabase/migrations)
- `--require-down` - Require down migrations
- `--check-naming` - Validate naming convention

### pre-push

Run all pre-push checks.

```bash
<binary> pre-push
<binary> pre-push --fail-fast
```

Options:
- `--fail-fast` - Stop on first failure
- `--skip <check>` - Skip specific checks

### doctor

Check environment health.

```bash
<binary> doctor
<binary> doctor --json
```

---

## iOS Commands (`fs-app`)

### format

Format Swift code using SwiftFormat.

```bash
fs-app format
fs-app format --staged    # Only staged files
fs-app format --check     # Check without modifying
```

Options:
- `--staged` - Only format staged files
- `--check` - Dry run, exit non-zero if changes needed
- `--config <path>` - SwiftFormat config file

### lint

Lint Swift code using SwiftLint.

```bash
fs-app lint
fs-app lint --strict
fs-app lint --fix
```

Options:
- `--strict` - Treat warnings as errors
- `--fix` - Auto-fix violations
- `--config <path>` - SwiftLint config file

### build

Build Xcode project.

```bash
fs-app build
fs-app build --configuration release
fs-app build --scheme MyScheme
```

Options:
- `--configuration <config>` - Build configuration (debug/release)
- `--scheme <name>` - Xcode scheme
- `--destination <dest>` - Build destination

### simulator

Manage iOS simulators.

```bash
fs-app simulator list
fs-app simulator boot --device "iPhone 15 Pro"
fs-app simulator shutdown
```

Subcommands:
- `list` - List available simulators
- `boot` - Boot a simulator
- `shutdown` - Shutdown running simulators

### project

Analyze Xcode project.

```bash
fs-app project status
fs-app project missing    # Files on disk not in project
fs-app project broken     # Broken file references
fs-app project sync       # Sync project with disk
```

---

## Android Commands (`fs-app`)

### format

Format Kotlin/Java code.

```bash
fs-app format
fs-app format --lang kotlin
fs-app format --staged
```

Options:
- `--lang <language>` - Language (kotlin/java)
- `--staged` - Only staged files

### lint

Run Android lint.

```bash
fs-app lint
fs-app lint --strict
```

### swift-core

Build Swift core library for Android.

```bash
fs-app swift-core build --target arm64
fs-app swift-core copy --output app/libs
```

Subcommands:
- `build` - Build Swift core
- `copy` - Copy built libraries

### emulator

Manage Android emulators.

```bash
fs-app emulator list
fs-app emulator boot pixel_7
fs-app emulator shutdown
```

---

## Web Commands (`lefthook-rs`)

### security

Run security checks.

```bash
lefthook-rs security
lefthook-rs security --json
```

### nextjs-security

Run Next.js-specific OWASP security checks.

```bash
lefthook-rs nextjs-security
lefthook-rs nextjs-security src/**/*.tsx
```

Checks:
- A01: Broken Access Control
- A02: Cryptographic Failures
- A03: Injection
- A07: XSS
- A10: SSRF

### bundle-size

Analyze bundle size.

```bash
lefthook-rs bundle-size
lefthook-rs bundle-size --threshold 500kb
```

Options:
- `--threshold <size>` - Fail if bundle exceeds size
- `--json` - Output as JSON

### conventional-commit

Validate conventional commit format.

```bash
lefthook-rs conventional-commit .git/COMMIT_MSG
```

---

## Global Options

All commands support:

- `--help` - Show help
- `--version` - Show version
- `--verbose` / `-v` - Increase verbosity
- `--quiet` / `-q` - Suppress output
- `--color <when>` - Color output (auto/always/never)
- `--json` - JSON output (where supported)
