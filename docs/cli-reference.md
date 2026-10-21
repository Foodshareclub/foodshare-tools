# CLI Reference

## Common Commands

These commands are available across all platform binaries (`fs-app`, `lefthook-rs`).

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

Check environment health (`fs-app` only).

```bash
fs-app doctor
fs-app doctor --json
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
lefthook-rs bundle-size --threshold 500    # Threshold in KB
```

Options:

- `--threshold <kb>` - Fail if bundle exceeds size in KB
- `--json` - Output as JSON

### conventional-commit

Validate conventional commit format.

```bash
lefthook-rs conventional-commit .git/COMMIT_MSG
```

### protected-branch

Check for pushes to protected branches.

```bash
lefthook-rs protected-branch
```

### large-files

Detect files exceeding size limits.

```bash
lefthook-rs large-files
lefthook-rs large-files --max-size 500    # Max size in KB
```

Options:

- `--max-size <kb>` - Maximum file size in KB (default: 500)

### accessibility

Check accessibility issues in JSX/TSX files.

```bash
lefthook-rs accessibility
lefthook-rs accessibility src/components/Button.tsx
```

### pre-commit

Run all pre-commit checks.

```bash
lefthook-rs pre-commit
lefthook-rs pre-commit src/app/page.tsx    # Check specific files
```

---

## Other Binaries

### foodshare-swift

Swift toolchain version management for the monorepo (detect, migrate, verify Swift versions).

```bash
foodshare-swift --help
foodshare-swift --project-root . --format text <subcommand>
```

### foodshare-migrate

Secret migration for self-hosted Supabase — syncs secrets between `.env.functions` and the Supabase Vault, and verifies required variables are present.

```bash
foodshare-migrate --help
foodshare-migrate --env-file .env.functions <subcommand>
```

### foodshare-i18n

Translation management CLI (health checks, audits, AI auto-translation, locale sync).

See [bins/foodshare-i18n/README.md](../bins/foodshare-i18n/README.md) for full documentation.

---

## Orchestration

Three binaries over one implementation, `foodshare-orchestrator`. See
[crates/orchestrator.md](./crates/orchestrator.md) for the library.

### fs-auto

Headless run-until-green: verify, build, clean.

```bash
fs-auto
fs-auto --quick          # Skip the WASM build
fs-auto --sequential     # Web and backend one after another
fs-auto --keep           # Keep the build artefacts
fs-auto --clean          # Clean before the run
fs-auto --retry 3        # Extra attempts per failing step
```

### fs-tui

The live build and test monitor. Interactive with no arguments; one
domain with a subcommand.

```bash
fs-tui                   # Interactive
fs-tui verify            # The whole plan, without the TUI
fs-tui rust              # One domain
fs-tui wasm | web | backend | mobile
```

Keys: `r` run the selected domain, `v` verify everything, `c` clean the
artefacts, `j`/`k` navigate, `q` quit.

### fs-orchestrator

The HTTP daemon, and the orchestrator from the command line.

```bash
fs-orchestrator serve --addr 127.0.0.1:7878
fs-orchestrator run --domain rust
fs-orchestrator run --domain all --quick --sequential
fs-orchestrator status
```

Options:

- `--addr <host:port>` - The address the daemon listens on
- `--root <path>` - The workspace root

The `run` subcommand takes `--domain <rust|wasm|web|backend|mobile|all>`,
`--quick` and `--sequential`.

Routes:

| Route                | Method | What it answers                          |
| -------------------- | ------ | ---------------------------------------- |
| `/healthz`           | GET    | The environment health of the machine     |
| `/readyz`            | GET    | Whether the daemon can do work            |
| `/metrics`           | GET    | The Prometheus exposition format          |
| `/status`            | GET    | The state of every domain                 |
| `/run/{domain}`      | POST   | Start a domain, or `all`                  |
| `/cancel`            | POST   | Stop the run in progress                  |

`POST /run/{domain}` starts the run and answers `202`. Add `?wait=true`
to wait for the result and get the outcome in the body.

### fs-maestro

The Maestro E2E runner: flows, syntax checks, suites and the device
matrix.

```bash
fs-maestro check              # show the device the suite would run on
fs-maestro syntax             # check every flow in the suite
fs-maestro smoke              # run the flows tagged `smoke`
fs-maestro regression         # run the flows tagged `regression`
fs-maestro critical           # run the flows tagged `critical`
fs-maestro a11y               # run the flows tagged `a11y`
fs-maestro test               # run the whole suite
fs-maestro flow <file>        # run one flow
fs-maestro report [format]    # run the suite and write a report
fs-maestro hierarchy          # dump the UI hierarchy
fs-maestro matrix [tag]       # run the suite on every device at once
```

Options:

- `--app-dir <path>` - The app repository
- `--device <id>` - The device to run on

### fs-wasm

The WebAssembly build engine.

```bash
fs-wasm                       # build every crate
fs-wasm --only geo            # build one crate
fs-wasm --json                # machine-readable output
```

Options:

- `--tools-dir <path>` - The Rust workspace
- `--web-dir <path>` - The web repository the modules go into

### fs-build

The cross-platform build engine: the Swift core, the iOS app and the
Android app.

```bash
fs-build                      # both platforms
fs-build --ios                # the iOS app
fs-build --android            # the Android app
fs-build --all                # both platforms
fs-build --swift              # the Swift core
fs-build --release            # a release build
fs-build --clean              # clean the artefacts first
fs-build swift                # the Swift unit tests
fs-build kotlin               # the Kotlin unit tests
fs-build bridge               # the Skip bridge tests
fs-build test                 # every test suite
```

### fs-sim

The simulator runner: boot, build, install, launch, screenshot.

```bash
fs-sim                        # the whole flow
fs-sim --smoke                # and the flows tagged `smoke`
fs-sim --test                 # and the whole Maestro suite
fs-sim --no-build             # reuse the existing bundle
fs-sim --clean                # clean the artefacts first
fs-sim --device=<udid>        # a specific simulator
```

### fs-xccloud

Xcode Cloud build runs, logs and diagnostics over the App Store Connect
API.

```bash
fs-xccloud status             # the recent build runs
fs-xccloud start [workflowId] # queue a build, and watch it
fs-xccloud watch [buildId]    # watch a build
fs-xccloud logs [buildId]     # the actions, issues and artefacts
fs-xccloud jwt                # print a signed token
fs-xccloud local              # the local build summaries
```

Options:

- `--app-dir <path>` - The app repository

### fs-verify

The full cross-repository verification.

```bash
fs-verify                      # every stage
fs-verify --skip-mobile        # without the simulator smoke tests
fs-verify --skip-node-guard    # without the toolchain policy check
fs-verify --json               # machine-readable output
```

The stages, in order: the Node.js guard in every repository, the disk,
the Rust workspace tests, the web build, the mobile smoke tests, and the
backend edge function tests. It stops at the first failure.

---

## Global Options

All commands support:

- `--help` - Show help
- `--version` - Show version
- `--verbose` / `-v` - Increase verbosity
- `--quiet` / `-q` - Suppress output
- `--color <when>` - Color output (auto/always/never)
- `--json` - JSON output (where supported)
