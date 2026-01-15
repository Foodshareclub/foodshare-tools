# foodshare-hooks

Git hook implementations for commit validation, secret scanning, and migration checks.

## Features

- Conventional commit message validation
- Secret/credential scanning (15+ patterns)
- Supabase migration validation
- Pre-push check aggregation

## Usage

```rust
use foodshare_hooks::{commit_msg, secrets, migrations};

// Validate commit message
commit_msg::validate("feat: add new feature")?;

// Scan for secrets
let findings = secrets::scan_staged_files()?;

// Check migrations
let issues = migrations::validate("supabase/migrations")?;
```

## Modules

### `commit_msg`

Conventional commit validation.

```rust
use foodshare_hooks::commit_msg::{validate, Config};

// Default validation
validate("feat: add login")?;

// Custom configuration
let config = Config {
    types: vec!["feat", "fix", "docs"],
    max_subject_length: 72,
    require_scope: false,
    ..Default::default()
};
validate_with_config("feat(auth): add login", &config)?;
```

### `secrets`

Secret and credential scanning.

Detected patterns:
- AWS Access Keys
- Supabase Keys
- GitHub Tokens
- Stripe Keys
- Generic API Keys
- Private Keys

### `migrations`

Supabase migration validation.

Checks:
- Naming convention
- Down migration presence
- SQL syntax validation
