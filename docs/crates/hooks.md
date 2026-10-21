# foodshare-hooks

Git hook implementations for commit validation, secret scanning, and migration checks.

## Features

- Conventional commit message validation
- Secret/credential scanning (15+ patterns)
- Supabase migration validation
- Pre-push check aggregation

## Usage

```rust
use foodshare_hooks::{commit_msg, migrations, secrets};
use foodshare_core::config::SecretsConfig;

// Validate a commit message file
let result = commit_msg::validate_commit_message(
    std::path::Path::new(".git/COMMIT_MSG"),
    &Default::default(),
)?;

// Scan files for secrets
let findings = secrets::scan_files(&[std::path::PathBuf::from("config.toml")], &SecretsConfig::default());

// Check the migration files
let check = migrations::check_migrations(std::path::Path::new("supabase/migrations"), true, true)?;
```

## Modules

### `commit_msg`

Conventional commit validation.

```rust
use foodshare_core::config::CommitMsgConfig;
use foodshare_hooks::commit_msg::validate_commit_message;

// Default validation
let result = validate_commit_message(
    std::path::Path::new(".git/COMMIT_MSG"),
    &CommitMsgConfig::default(),
)?;

// Custom configuration
let config = CommitMsgConfig {
    types: vec!["feat", "fix", "docs"],
    max_length: 72,
    ..Default::default()
};
let result = validate_commit_message(std::path::Path::new(".git/COMMIT_MSG"), &config)?;
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
