# Configuration

Foodshare Tools uses TOML configuration files for customization.

## Configuration File

Configuration is discovered relative to the current working directory. The first file found wins:

1. `.foodshare-hooks.toml`
2. `foodshare-hooks.toml`
3. `.config/foodshare-hooks.toml`

There is no user-level (`~/.config`) configuration file and no environment-variable overrides for configuration values.

Example:

```toml
# .foodshare-hooks.toml

[commit_msg]
# Allowed commit types
types = ["feat", "fix", "docs", "style", "refactor", "test", "chore", "perf", "ci", "build", "revert"]

# Maximum subject line length
max_length = 72

# Minimum subject line length
min_length = 10

# Skip validation for merge commits
skip_merge = true

# Skip validation for revert commits
skip_revert = true

[secrets]
# Additional patterns to detect (regex)
additional_patterns = []

# Patterns to exclude (e.g., placeholder values)
exclude_patterns = [
    "EXAMPLE_",
    "PLACEHOLDER_",
    "YOUR_",
    "xxx",
    "test_"
]

# Files to exclude from secret scanning
exclude_files = [
    "*.test.ts",
    "*.spec.ts",
    "*.mock.ts",
    "*.fixture.ts",
    "__mocks__/**",
    "**/__tests__/**"
]
```

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
| `FOODSHARE_PARALLEL`     | Run checks in parallel                                                       |
| `FOODSHARE_QUICK_MODE`   | Enable quick mode (skip slow checks)                                         |
| `FOODSHARE_STRICT`       | Treat warnings as errors                                                     |
| `FOODSHARE_TELEMETRY`    | Enable/disable telemetry                                                     |
| `FOODSHARE_TIMEOUT_SECS` | Operation timeout in seconds                                                 |

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

Built-in patterns detect:

| Pattern               | Example                                   |
| --------------------- | ----------------------------------------- |
| AWS Access Key        | `AKIA...`                                 |
| AWS Secret Key        | `aws_secret_access_key = "..."`           |
| Supabase Anon Key     | `eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...` |
| Supabase Service Role | `service_role` key pattern                |
| GitHub Token          | `ghp_...`, `gho_...`, `ghu_...`           |
| Stripe Key            | `sk_live_...`, `pk_live_...`              |
| Generic API Key       | `api_key`, `apikey`, `api-key` patterns   |
| Private Key           | PEM headers containing `PRIVATE KEY`      |
| Password in URL       | `://user:password@`                       |

Add your own patterns:

```toml
[secrets]
additional_patterns = [
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
