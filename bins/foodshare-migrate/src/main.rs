//! Foodshare Migrate CLI — secret migration for self-hosted Supabase
//!
//! Syncs secrets between `.env.functions` and the Supabase vault,
//! and ensures all required environment variables are present.

use clap::{Parser, Subcommand};
use foodshare_cli::output::Status;
use foodshare_cli::progress;
use foodshare_migrate::{env_file, secrets, vault::VaultClient};
use owo_colors::OwoColorize;
use std::path::PathBuf;
use std::process::ExitCode;

/// Secret migration tool for self-hosted Supabase
#[derive(Parser)]
#[command(name = "foodshare-migrate")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Path to the edge-function env file
    #[arg(long, global = true, default_value = ".env.functions")]
    env_file: PathBuf,

    /// Docker container name for the Supabase database
    #[arg(long, global = true, default_value = "supabase-db")]
    container: String,

    /// Postgres user inside the container
    #[arg(long, global = true, default_value = "supabase_admin")]
    db_user: String,

    /// Show what would happen without making changes
    #[arg(long, global = true)]
    dry_run: bool,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Vault secret operations
    Vault {
        #[command(subcommand)]
        action: VaultAction,
    },
    /// Environment file operations
    Env {
        #[command(subcommand)]
        action: EnvAction,
    },
    /// Show full migration status (vault + env vars)
    Status,
    /// Run full migration: vault sync + env sync + verify
    Run,
}

#[derive(Subcommand)]
enum VaultAction {
    /// Read from environment or .env file and create/update vault secrets
    Sync {
        /// Use environment variables as the source instead of the .env file
        #[arg(long)]
        from_env: bool,

        /// Sync all variables from the source, not just the hardcoded list
        #[arg(long)]
        all: bool,

        /// Only sync variables with this prefix (e.g. FS_SECRET_)
        #[arg(long)]
        prefix: Option<String>,
    },
    /// Set a specific secret in the vault
    Set {
        /// Name of the secret
        key: String,
        /// Value of the secret
        value: String,
        /// Human-readable description
        #[arg(long, default_value = "Manual secret update")]
        description: String,
    },
    /// List all vault secrets
    List,
    /// Test PG functions that read from vault
    Verify,
}

#[derive(Subcommand)]
enum EnvAction {
    /// Append missing variables to .env.functions
    Sync {
        /// Populates the environment file from the Supabase Vault instead of known defaults
        #[arg(long)]
        from_vault: bool,
    },
    /// Show what would be added (dry run)
    Diff,
    /// Update or append a variable in the env file
    Set {
        /// Variable name (e.g. JWT_SECRET)
        key: String,
        /// Variable value
        value: String,
    },
    /// Dump all vault secrets into .env format to stdout
    Dump,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("foodshare_migrate=debug")
            .init();
    }

    let result = match &cli.command {
        Commands::Vault { action } => match action {
            VaultAction::Sync {
                from_env,
                all,
                prefix,
            } => cmd_vault_sync(&cli, *from_env, *all, prefix.as_deref()),
            VaultAction::Set {
                key,
                value,
                description,
            } => cmd_vault_set(&cli, key, value, description),
            VaultAction::List => cmd_vault_list(&cli),
            VaultAction::Verify => cmd_vault_verify(&cli),
        },
        Commands::Env { action } => match action {
            EnvAction::Sync { from_vault } => cmd_env_sync(&cli, *from_vault),
            EnvAction::Diff => cmd_env_diff(&cli),
            EnvAction::Set { key, value } => cmd_env_set(&cli, key, value),
            EnvAction::Dump => cmd_env_dump(&cli),
        },
        Commands::Status => cmd_status(&cli),
        Commands::Run => cmd_run(&cli),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// vault sync
// ---------------------------------------------------------------------------

fn cmd_vault_sync(
    cli: &Cli,
    from_env: bool,
    all: bool,
    prefix: Option<&str>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    Status::header("Vault Sync");

    let client = make_vault_client(cli)?;

    let env_vars = if from_env {
        Status::info("Source: System environment variables");
        std::env::vars().collect::<std::collections::HashMap<String, String>>()
    } else {
        Status::info(&format!("Source: {}", cli.env_file.display()));
        env_file::parse_env_file(&cli.env_file)?
    };

    let mut created = 0u32;
    let mut updated = 0u32;
    let mut skipped = 0u32;
    let mut missing = 0u32;

    // 1. Hardcoded secrets
    let pb = progress::progress_bar(secrets::VAULT_SECRETS.len() as u64, "Syncing vault secrets");

    for secret in secrets::VAULT_SECRETS {
        pb.inc(1);

        let value = match env_vars.get(secret.env_key) {
            Some(v) if !v.is_empty() => v.clone(),
            _ => {
                if !all && prefix.is_none() {
                    Status::warning(&format!(
                        "No value for {} in source — skipping {}",
                        secret.env_key, secret.vault_name,
                    ));
                }
                missing += 1;
                continue;
            }
        };

        if client.secret_exists(secret.vault_name)? {
            if cli.dry_run {
                Status::info(&format!("Would update: {}", secret.vault_name));
            } else {
                client.update_secret(&value, secret.vault_name)?;
                Status::success(&format!("Updated: {}", secret.vault_name));
            }
            updated += 1;
        } else {
            if cli.dry_run {
                Status::info(&format!("Would create: {}", secret.vault_name));
            } else {
                client.create_secret(&value, secret.vault_name, secret.description)?;
                Status::success(&format!("Created: {}", secret.vault_name));
            }
            created += 1;
        }
    }

    progress::finish_success(&pb, "Hardcoded sync done");

    // 2. Dynamic sync (if --all or --prefix)
    if all || prefix.is_some() {
        Status::subheader("Dynamic Sync");
        let pb_dynamic =
            progress::progress_bar(env_vars.len() as u64, "Scanning source for dynamic secrets");

        for (key, value) in &env_vars {
            pb_dynamic.inc(1);

            // Skip if it's already in hardcoded list
            if secrets::VAULT_SECRETS.iter().any(|s| s.env_key == key) {
                continue;
            }

            // check prefix
            if let Some(p) = prefix {
                if !key.starts_with(p) {
                    continue;
                }
            } else if !all {
                continue;
            }

            // Exclude common system/shell vars if using --all from env
            if all && from_env {
                let excluded = [
                    "PATH",
                    "HOME",
                    "USER",
                    "PWD",
                    "SHELL",
                    "LS_COLORS",
                    "_",
                    "SSH_CLIENT",
                    "SSH_CONNECTION",
                    "SSH_TTY",
                    "SSH_AUTH_SOCK",
                    "LANG",
                    "LC_ALL",
                    "LANGUAGE",
                    "DEBIAN_FRONTEND",
                    "TERM",
                    "MAIL",
                    "OLDPWD",
                    "SHLVL",
                    "MOTD_SHOWN",
                    "XDG_SESSION_ID",
                    "XDG_RUNTIME_DIR",
                    "S_COLORS",
                ];
                if excluded.contains(&key.as_str()) {
                    continue;
                }
            }

            if value.is_empty() {
                continue;
            }

            if client.secret_exists(key)? {
                if cli.dry_run {
                    Status::info(&format!("Would update (dynamic): {}", key));
                } else {
                    client.update_secret(value, key)?;
                    Status::success(&format!("Updated (dynamic): {}", key));
                }
                updated += 1;
            } else {
                if cli.dry_run {
                    Status::info(&format!("Would create (dynamic): {}", key));
                } else {
                    client.create_secret(value, key, "Dynamic secret via migrate tool")?;
                    Status::success(&format!("Created (dynamic): {}", key));
                }
                created += 1;
            }
        }
        progress::finish_success(&pb_dynamic, "Dynamic sync done");
    }

    println!();
    if cli.dry_run {
        Status::info(&format!(
            "Dry run: would create {created}, update {updated}, missing from source: {missing}"
        ));
    } else {
        Status::success(&format!(
            "Created: {created}, Updated: {updated}, Missing from source: {missing}"
        ));
    }

    Ok(())
}

fn cmd_vault_set(
    cli: &Cli,
    key: &str,
    value: &str,
    description: &str,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    Status::header("Vault Set");
    let client = make_vault_client(cli)?;

    if client.secret_exists(key)? {
        if cli.dry_run {
            Status::info(&format!("Would update: {} (dry run)", key));
        } else {
            client.update_secret(value, key)?;
            Status::success(&format!("Updated: {}", key));
        }
    } else {
        if cli.dry_run {
            Status::info(&format!("Would create: {} (dry run)", key));
        } else {
            client.create_secret(value, key, description)?;
            Status::success(&format!("Created: {}", key));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// vault list
// ---------------------------------------------------------------------------

fn cmd_vault_list(cli: &Cli) -> std::result::Result<(), Box<dyn std::error::Error>> {
    Status::header("Vault Secrets");

    let client = make_vault_client(cli)?;
    let names = client.list_secrets()?;

    if names.is_empty() {
        Status::warning("No secrets found in vault");
    } else {
        for name in &names {
            println!("  {}", name);
        }
        println!();
        Status::info(&format!("{} secrets in vault", names.len()));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// vault verify
// ---------------------------------------------------------------------------

fn cmd_vault_verify(cli: &Cli) -> std::result::Result<(), Box<dyn std::error::Error>> {
    Status::header("Vault Verification");

    let client = make_vault_client(cli)?;
    let mut all_ok = true;

    for fn_name in secrets::VERIFY_FUNCTIONS {
        match client.verify_function(fn_name)? {
            Some(_) => {
                Status::success(&format!("{fn_name}() — returns non-null"));
            }
            None => {
                Status::error(&format!("{fn_name}() — returns NULL"));
                all_ok = false;
            }
        }
    }

    // Also verify MOTHERDUCK_TOKEN via get_vault_secret
    match client.verify_function("get_vault_secret") {
        Ok(_) => {
            // Function exists, test with specific secret
            let result = client.run_vault_secret_check("MOTHERDUCK_TOKEN")?;
            if result {
                Status::success("get_vault_secret('MOTHERDUCK_TOKEN') — returns non-null");
            } else {
                Status::error("get_vault_secret('MOTHERDUCK_TOKEN') — returns NULL");
                all_ok = false;
            }
        }
        Err(_) => {
            Status::warning("get_vault_secret() function not found, skipping");
        }
    }

    println!();
    if all_ok {
        Status::success("All verification checks passed");
    } else {
        Status::error("Some verification checks failed");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// env sync
// ---------------------------------------------------------------------------

fn cmd_env_sync(
    cli: &Cli,
    from_vault: bool,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    Status::header("Env File Sync");
    let existing = env_file::parse_env_file(&cli.env_file)?;

    let missing = if from_vault {
        Status::info("Source: Supabase Vault");
        let client = make_vault_client(cli)?;
        let vault_secrets = client.get_decrypted_secrets()?;

        // Compute which vault secrets are missing from the env file
        let mut to_add = Vec::new();
        for (name, value) in vault_secrets {
            if !existing.contains_key(&name) {
                to_add.push((name, value));
            }
        }
        to_add
    } else {
        Status::info("Source: Known defaults");
        env_file::compute_missing_vars(&existing)
    };

    if missing.is_empty() {
        Status::success("All source variables are already present in the env file");
        return Ok(());
    }

    if cli.dry_run {
        Status::info(&format!(
            "Would append {} variables (use without --dry-run to apply)",
            missing.len()
        ));
        for (key, _) in &missing {
            println!("  {} {key}", "+".green());
        }
        return Ok(());
    }

    let result = env_file::append_missing_vars(&cli.env_file, &missing)?;
    Status::success(&format!(
        "Appended {} variables, skipped {} existing",
        result.appended, result.skipped
    ));

    Ok(())
}

// ---------------------------------------------------------------------------
// env diff
// ---------------------------------------------------------------------------

fn cmd_env_diff(cli: &Cli) -> std::result::Result<(), Box<dyn std::error::Error>> {
    Status::header("Env File Diff");

    let existing = env_file::parse_env_file(&cli.env_file)?;
    let missing = env_file::compute_missing_vars(&existing);

    if missing.is_empty() {
        Status::success("All known env vars are present — nothing to add");
        return Ok(());
    }

    let groups = env_file::missing_env_var_groups();
    for group in &groups {
        let group_missing: Vec<_> = group
            .vars
            .iter()
            .filter(|(key, _)| !existing.contains_key(*key))
            .collect();

        if group_missing.is_empty() {
            continue;
        }

        Status::subheader(&format!("[{}]", group.section));
        for (key, default) in group_missing {
            println!("  {} {key}={default}", "+".green());
        }
    }

    println!();
    Status::info(&format!("{} variables would be added", missing.len()));

    Ok(())
}

// ---------------------------------------------------------------------------
// status
// ---------------------------------------------------------------------------

fn cmd_status(cli: &Cli) -> std::result::Result<(), Box<dyn std::error::Error>> {
    Status::header("Migration Status");

    // Env file status
    Status::subheader("Environment File");
    match env_file::parse_env_file(&cli.env_file) {
        Ok(existing) => {
            Status::success(&format!(
                "{} — {} variables loaded",
                cli.env_file.display(),
                existing.len()
            ));

            let missing = env_file::compute_missing_vars(&existing);
            if missing.is_empty() {
                Status::success("All known env var groups are complete");
            } else {
                Status::warning(&format!(
                    "{} env vars missing from known groups",
                    missing.len()
                ));
            }

            // Check vault secret source vars
            let mut vault_ready = 0;
            let mut vault_missing = 0;
            for secret in secrets::VAULT_SECRETS {
                match existing.get(secret.env_key) {
                    Some(v) if !v.is_empty() => vault_ready += 1,
                    _ => vault_missing += 1,
                }
            }
            Status::info(&format!(
                "Vault source values: {vault_ready} ready, {vault_missing} missing"
            ));
        }
        Err(e) => {
            Status::error(&format!("{} — {e}", cli.env_file.display()));
        }
    }

    // Vault status
    Status::subheader("Vault");
    let client = make_vault_client(cli);
    match client {
        Ok(client) => {
            match client.list_secrets() {
                Ok(names) => {
                    Status::success(&format!("{} secrets in vault", names.len()));

                    // Check which required secrets exist
                    let required: Vec<_> = secrets::VAULT_SECRETS
                        .iter()
                        .map(|s| s.vault_name)
                        .collect();
                    let present: Vec<_> = required
                        .iter()
                        .filter(|name| names.iter().any(|n| n == **name))
                        .collect();
                    let absent: Vec<_> = required
                        .iter()
                        .filter(|name| !names.iter().any(|n| n == **name))
                        .collect();

                    if absent.is_empty() {
                        Status::success("All 12 required vault secrets present");
                    } else {
                        Status::warning(&format!(
                            "{} of 12 required secrets present, {} missing",
                            present.len(),
                            absent.len()
                        ));
                        for name in &absent {
                            println!("    {} {name}", "missing:".yellow());
                        }
                    }
                }
                Err(e) => Status::error(&format!("Failed to list secrets: {e}")),
            }

            // PG function verification
            Status::subheader("PG Functions");
            for fn_name in secrets::VERIFY_FUNCTIONS {
                match client.verify_function(fn_name) {
                    Ok(Some(_)) => Status::success(&format!("{fn_name}() — OK")),
                    Ok(None) => Status::error(&format!("{fn_name}() — returns NULL")),
                    Err(e) => Status::error(&format!("{fn_name}() — error: {e}")),
                }
            }
        }
        Err(e) => {
            Status::error(&format!("Cannot connect to vault: {e}"));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// run (full migration)
// ---------------------------------------------------------------------------

fn cmd_run(cli: &Cli) -> std::result::Result<(), Box<dyn std::error::Error>> {
    Status::header("Full Migration (Vault-First)");

    Status::step(1, 4, "Syncing source to Vault...");
    // By default, sync all from the specified env file to vault
    cmd_vault_sync(cli, false, true, None)?;

    Status::step(2, 4, "Syncing Vault to environment file...");
    // Populate the env file with everything currently in the Vault
    cmd_env_sync(cli, true)?;

    Status::step(3, 4, "Ensuring mandatory env variables...");
    // Fallback to defaults for missing infra variables (POSTGRES_PASSWORD etc)
    cmd_env_sync(cli, false)?;

    Status::step(4, 4, "Verifying Vault integration...");
    cmd_vault_verify(cli)?;

    println!();
    Status::success("Migration complete. Run `docker compose restart functions` to apply.");

    Ok(())
}

// ---------------------------------------------------------------------------
// env set
// ---------------------------------------------------------------------------

fn cmd_env_set(
    cli: &Cli,
    key: &str,
    value: &str,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    Status::header("Env File Set");

    if cli.dry_run {
        Status::info(&format!(
            "Would set {}={} in {} (dry run)",
            key,
            value,
            cli.env_file.display()
        ));
    } else {
        env_file::upsert_var(&cli.env_file, key, value)?;
        Status::success(&format!("Updated {} in {}", key, cli.env_file.display()));
    }

    Ok(())
}

fn cmd_env_dump(cli: &Cli) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let client = make_vault_client(cli)?;
    let secrets = client.get_decrypted_secrets()?;

    let mut keys: Vec<_> = secrets.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(value) = secrets.get(key) {
            println!("{key}={value}");
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn make_vault_client(cli: &Cli) -> std::result::Result<VaultClient, Box<dyn std::error::Error>> {
    let client = VaultClient::new(&cli.container, &cli.db_user, "postgres");
    client.check_connection()?;
    Ok(client)
}

/// Extension trait to check a specific vault secret via `get_vault_secret()`.
trait VaultClientExt {
    fn run_vault_secret_check(
        &self,
        secret_name: &str,
    ) -> std::result::Result<bool, Box<dyn std::error::Error>>;
}

impl VaultClientExt for VaultClient {
    fn run_vault_secret_check(
        &self,
        secret_name: &str,
    ) -> std::result::Result<bool, Box<dyn std::error::Error>> {
        // Validate the secret name (alphanumeric + underscore only)
        if !secret_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(Box::new(
                foodshare_migrate::error::MigrateError::verification(format!(
                    "invalid secret name: {secret_name}"
                )),
            ));
        }

        let exists = self.secret_exists(secret_name)?;
        Ok(exists)
    }
}
