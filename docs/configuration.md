# Configuration

Foodshare Tools uses TOML configuration files for customization.

## Configuration File

Create `.foodshare-hooks.toml` in your project root:

```toml
# .foodshare-hooks.toml

[commit_msg]
# Allowed commit types
types = ["feat", "fix", "docs", "style", "refactor", "test", "chore", "ci", "perf"]

# Maximum subject line length
max_subject_length = 72

# Require scope in commit message (e.g., feat(auth): ...)
require_scope = false

# Allowed scopes (empty = any scope allowed)
scopes = []

[secrets]
# Files to exclude from secret scanning
exclude_files = [
    "*.test.ts",
    "*.spec.ts",
    "*.mock.ts",
    "*.fixture.ts",
    "__mocks__/**",
    "**/__tests__/**"
]

# Patterns to exclude (e.g., placeholder values)
exclude_patterns = [
    "EXAMPLE_",
    "PLACEHOLDER_",
    "YOUR_",
    "xxx",
    "test_"
]

# Additional patterns to detect (regex)
custom_patterns = []

# Severity level (error/warning)
severity = "error"

[migrations]
# Migrations directory
directory = "supabase/migrations"

# Require down migrations
require_down = true

# Check naming convention (YYYYMMDDHHMMSS_name.sql)
check_naming = true

# Allowed SQL statements in migrations
allowed_statements = ["CREATE", "ALTER", "DROP", "INSERT", "UPDATE", "DELETE"]

[format]
# SwiftFormat configuration (iOS)
swift_config = ".swiftformat"

# Kotlin format configuration (Android)
kotlin_config = ".editorconfig"

[lint]
# SwiftLint configuration (iOS)
swift_config = ".swiftlint.yml"

# Treat warnings as errors
strict = false

[build]
# Default build configuration
configuration = "debug"

# Default scheme (iOS)
scheme = ""

# Parallel jobs
jobs = 0  # 0 = auto-detect

[telemetry]
# Enable telemetry
enabled = false

# Log level (trace/debug/info/warn/error)
log_level = "info"

# Metrics endpoint
metrics_endpoint = ""
```

## Environment Variables

Configuration can be overridden with environment variables:

| Variable | Description |
|----------|-------------|
| `FOODSHARE_CONFIG` | Path to config file |
| `FOODSHARE_LOG_LEVEL` | Log level (trace/debug/info/warn/error) |
| `FOODSHARE_NO_COLOR` | Disable colored output |
| `FOODSHARE_JSON` | Enable JSON output |

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
    "ci",       # CI/CD changes
    "perf",     # Performance improvement
    "revert",   # Revert previous commit
    "build",    # Build system changes
]
```

### Secret Patterns

Built-in patterns detect:

| Pattern | Example |
|---------|---------|
| AWS Access Key | `AKIA...` |
| AWS Secret Key | `aws_secret_access_key = "..."` |
| Supabase Anon Key | `eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...` |
| Supabase Service Role | `service_role` key pattern |
| GitHub Token | `ghp_...`, `gho_...`, `ghu_...` |
| Stripe Key | `sk_live_...`, `pk_live_...` |
| Generic API Key | `api_key`, `apikey`, `api-key` patterns |
| Private Key | `-----BEGIN RSA PRIVATE KEY-----` |
| Password in URL | `://user:password@` |

Add custom patterns:

```toml
[secrets]
custom_patterns = [
    "my_company_[a-zA-Z0-9]{32}",
    "internal_token_[0-9a-f]{64}"
]
```

### Migration Validation

```toml
[migrations]
# Strict naming: YYYYMMDDHHMMSS_description.sql
check_naming = true

# Require corresponding down migration
require_down = true

# Directory structure
directory = "supabase/migrations"
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

1. Command-line arguments (highest)
2. Environment variables
3. Project config (`.foodshare-hooks.toml`)
4. User config (`~/.config/foodshare/config.toml`)
5. Default values (lowest)
