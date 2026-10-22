# Configuration

Foodshare Tools uses TOML configuration files for customization.

## Configuration File

Configuration is discovered relative to the current working directory. The first file found wins:

1. `.foodshare-hooks.toml`
2. `foodshare-hooks.toml`
3. `.config/foodshare-hooks.toml`

There is no user-level (`~/.config`) configuration file and no environment-variable overrides for configuration values.

## What this build reads

These are the sections the tools act on. A section that is not here is
not implemented by this build, and the loader says so — see [Unknown
keys](#unknown-keys) below.

| Section      | What it does                                              |
| ------------ | --------------------------------------------------------- |
| `general`    | The project name, the scheme, the destination, the device, the bundle identifier, the package name, the Swift version, the source directory, and the paths to exclude |
| `commit_msg` | The commit-message format: the types, the length bounds, and the merge and revert skips |
| `analyze`    | The thresholds for TODOs, FIXMEs and force unwraps, and the paths to analyze |
| `secrets`    | The extra patterns to look for, the patterns to excuse, and the files to skip |
| `format`     | Whether to stage what was formatted, and the paths to format |
| `lint`       | Whether warnings are failures, and the paths to lint           |
| `migrations` | Where the migration files live                             |
| `pre_push`   | Whether to stop at the first failure, the build configuration, and the deadlines |

Example:

```toml
# .foodshare-hooks.toml

[general]
scheme = "FoodShare"
destination = "platform=iOS Simulator,name=iPhone 17 Pro Max"
device = "iPhone 17 Pro Max"
bundle_id = "com.flutterflow.foodshare"
package_name = "com.foodshare"
exclude_paths = ["**/Build/**", "**/DerivedData/**"]

[commit_msg]
# Allowed commit types
types = ["feat", "fix", "docs", "style", "refactor", "test", "chore", "perf", "ci", "build", "revert"]

# Subject line length bounds
max_length = 72
min_length = 10

# Skip validation for merge and revert commits
skip_merge = true
skip_revert = true

[secrets]
# Extra patterns to detect, as regular expressions.
#
# These are anchored to an assignment on purpose. A bare keyword is not
# a pattern: a repository talks about tokens, secrets and credentials
# in prose, in source comments and in translation strings, so a list of
# plain words matched every one of those lines. A guard that reports
# everything is a guard that gets turned off, and then it reports
# nothing.
patterns = [
    '(?i)\b(secret|credential|token)\b\s*[=:]\s*["\'][^$"\'\n][^"\'\n]{7,}["\']',
]

# Lines that are known to hold a placeholder. The value is matched
# anywhere in the line, so only the placeholder line is skipped and a
# real secret on another line is still found.
exclude_patterns = [
    "your_token_here",
]

# Files to exclude from secret scanning. Matched as a substring of the
# path, not as a glob.
exclude_files = [
    ".lefthook.yml",
    "compose.yaml",
]
```

## Unknown keys

A section or a key that this build does not implement is reported, on
stderr, when the configuration is read:

```
⚠ .foodshare-hooks.toml: 3 configuration key(s) are not implemented by this tool and were ignored:
  android.module
  pre_push.full_checks
  version.project_file
```

The report never fails a command. A repository with a section the tool
does not read should still be able to run the tool; it should just
learn that the section does nothing.

The sections the estate writes that no tool reads today are `[android]`,
`[build]`, `[clean]`, `[deps]`, `[docs]`, `[test]` and `[version]`. They
are in the report on purpose: they are the ones somebody would otherwise
believe were doing something.

There is a test that keeps this honest: `crates/core/tests/schema_is_read.rs`
fails when the schema declares a key that nothing reads. A key that is
declared but never read is invisible to the report — the report would
say the key is implemented, and it would not be — so the schema and the
implementation have to be kept in step by something.

## Environment Variables

Binaries recognize the following environment variables at runtime (these are not configuration-file overrides):

| Variable                 | Description                                                                  |
| ------------------------ | ---------------------------------------------------------------------------- |
| `FOODSHARE_API_URL`      | API endpoint URL                                                             |
| `FOODSHARE_BACKEND_DIR`  | Backend directory                                                            |
| `FOODSHARE_BFF_URL`      | BFF endpoint URL                                                             |
| `FOODSHARE_CACHE`        | Cache directory                                                              |
| `FOODSHARE_COLOR`        | Color output (auto/always/never)                                             |
| `FOODSHARE_DEBUG`        | Enable debug output                                                          |
| `FOODSHARE_ENV`          | Environment name                                                             |
| `FOODSHARE_HOOKS_BIN`    | Path to the `lefthook-rs` binary invoked by centralized `lefthook.yml` gates |
| `FOODSHARE_ORCHESTRATOR_TOKEN` | Token the daemon's control routes require                           |
| `FOODSHARE_ORCHESTRATOR_URL`  | Where the daemon is running                                            |
| `FOODSHARE_PARALLEL`     | Run checks in parallel                                                       |
| `FOODSHARE_QUICK_MODE`   | Enable quick mode (skip slow checks)                                         |
| `FOODSHARE_STEP_TIMEOUT_SECS` | Timeout for a step, in seconds                                        |
| `FOODSHARE_STRICT`       | Treat warnings as errors                                                     |
| `FOODSHARE_TELEMETRY`    | Enable/disable telemetry                                                     |
| `FOODSHARE_TIMEOUT_SECS` | Operation timeout in seconds                                                 |
| `FOODSHARE_WORKSPACE_ROOT` | Where the workspace is                                               |
| `ANDROID_HOME`           | The Android SDK, for the mobile steps                                        |
| `ANDROID_SDK_ROOT`       | The Android SDK, for the mobile steps                                        |

## Per-Command Configuration

### Commit Message Types

```toml
[commit_msg]
types = [
    "feat",     # New feature
    "fix",      # Bug fix
    "docs",     # Documentation
    "style",    # Formatting, no code change
    "refactor", # Code restructuring
    "test",     # Adding tests
    "chore",    # Maintenance
    "perf",     # Performance improvement
    "ci",       # CI/CD changes
    "build",    # Build system changes
    "revert",   # Revert previous commit
]

# Subject length bounds and merge/revert skipping
max_length = 72
min_length = 10
skip_merge = true
skip_revert = true
```

### Secret Patterns

The built-in patterns are twenty:

| Pattern                    | Example                                      |
| -------------------------- | -------------------------------------------- |
| AWS access key             | `AKIA...`                                    |
| AWS secret key             | `aws_secret_access_key = "..."`              |
| Google API key             | `AIza...`                                    |
| Firebase URL               | a Firebase database URL                      |
| Heroku API key             | a Heroku API key                             |
| GitHub token               | `ghp_...`, `gho_...`, `ghu_...`              |
| npm token                  | `npm_...`                                    |
| PyPI token                 | `pypi-...`                                   |
| Database URL               | a URL with a password in it                  |
| Supabase key               | a Supabase anon or service-role key          |
| Stripe secret key          | `sk_live_...`, `sk_test_...`                 |
| Slack webhook              | a Slack webhook URL                          |
| Discord webhook            | a Discord webhook URL                        |
| Standard Webhooks secret   | `whsec_...`                                  |
| Twilio auth token          | a Twilio auth token                          |
| SendGrid API key           | `SG. ...`                                    |
| Private key                | PEM headers containing `PRIVATE KEY`         |
| Password assignment        | `password = "..."`                           |
| Generic API key            | `api_key`, `apikey`, `api-key` patterns      |
| Debug print                | a `print` that looks like it prints a secret |
| Custom token               | a token-shaped value                         |

The scanner also looks for a line that is high-entropy, whatever it
looks like.

Add your own patterns:

```toml
[secrets]
patterns = [
    "my_company_[a-zA-Z0-9]{32}",
    "internal_token_[0-9a-f]{64}"
]
```

## Platform-Specific Configuration

### iOS (.swiftformat)

```
--swiftversion 5.9
--indent 4
--indentcase false
--trimwhitespace always
--voidtype void
--semicolons never
--header strip
```

### iOS (.swiftlint.yml)

```yaml
disabled_rules:
  - trailing_whitespace
opt_in_rules:
  - empty_count
  - closure_spacing
line_length: 120
```

### Android (.editorconfig)

```ini
[*.kt]
indent_size = 4
max_line_length = 120
```

## Configuration Precedence

1. Command-line flags (highest)
2. Config file discovered relative to the current working directory (`.foodshare-hooks.toml`, then `foodshare-hooks.toml`, then `.config/foodshare-hooks.toml`)
3. Built-in default values (lowest)

There is no user-level config path and no environment-variable overrides for configuration values.
