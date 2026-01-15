# motherduck-sync

MotherDuck database synchronization utility.

## Features

- Sync data between Supabase and MotherDuck
- Incremental updates
- Schema management
- CLI and library usage

## CLI Usage

```bash
# Sync all tables
motherduck-sync sync

# Sync specific table
motherduck-sync sync --table posts

# Check sync status
motherduck-sync status

# Initialize schema
motherduck-sync init
```

## Configuration

```toml
# supamigrate.toml

[source]
type = "supabase"
url = "${SUPABASE_URL}"
key = "${SUPABASE_SERVICE_ROLE_KEY}"

[destination]
type = "motherduck"
token = "${MOTHERDUCK_TOKEN}"
database = "foodshare"

[sync]
tables = ["posts", "profiles", "rooms"]
batch_size = 1000
incremental = true
```

## Library Usage

```rust
use motherduck_sync::{Sync, Config};

let config = Config::from_file("supamigrate.toml")?;
let sync = Sync::new(config)?;

// Full sync
sync.run().await?;

// Sync specific table
sync.sync_table("posts").await?;

// Get status
let status = sync.status().await?;
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `SUPABASE_URL` | Supabase project URL |
| `SUPABASE_SERVICE_ROLE_KEY` | Service role key |
| `MOTHERDUCK_TOKEN` | MotherDuck API token |
