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
    /// Read .env.functions and create missing vault secrets
    Sync,
    /// List all vault secrets
    List,
    /// Test PG functions that read from vault
    Verify,
}

#[derive(Subcommand)]
enum EnvAction {
    /// Append missing variables to .env.functions
    Sync,
    /// Show what would be added (dry run)
    Diff,
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
            VaultAction::Sync => cmd_vault_sync(&cli),
            VaultAction::List => cmd_vault_list(&cli),
            VaultAction::Verify => cmd_vault_verify(&cli),
        },
        Commands::Env { action } => match action {
            EnvAction::Sync => cmd_env_sync(&cli),
            EnvAction::Diff => cmd_env_diff(&cli),
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

fn cmd_vault_sync(cli: &Cli) -> std::result::Result<(), Box<dyn std::error::Error>> {
    Status::header("Vault Sync");

    let client = make_vault_client(cli)?;
    let env_vars = env_file::parse_env_file(&cli.env_file)?;

    let mut created = 0u32;
    let mut skipped = 0u32;
    let mut missing = 0u32;

    let pb = progress::progress_bar(
        secrets::VAULT_SECRETS.len() as u64,
        "Syncing vault secrets",
    );

    for secret in secrets::VAULT_SECRETS {
        pb.inc(1);

        let value = match env_vars.get(secret.env_key) {
            Some(v) if !v.is_empty() => v.clone(),
            _ => {
                Status::warning(&format!(
                    "No value for {} in {} — skipping {}",
                    secret.env_key,
                    cli.env_file.display(),
                    secret.vault_name,
                ));
                missing += 1;
                continue;
            }
        };

        if client.secret_exists(secret.vault_name)? {
            skipped += 1;
            continue;
        }

        if cli.dry_run {
            Status::info(&format!("Would create: {}", secret.vault_name));
            created += 1;
        } else {
            client.create_secret(&value, secret.vault_name, secret.description)?;
            Status::success(&format!("Created: {}", secret.vault_name));
            created += 1;
        }
    }

    progress::finish_success(&pb, "Done");

    println!();
    if cli.dry_run {
        Status::info(&format!(
            "Dry run: would create {created}, skip {skipped}, missing from env: {missing}"
        ));
    } else {
        Status::success(&format!(
            "Created: {created}, Skipped: {skipped}, Missing from env: {missing}"
        ));
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

fn cmd_env_sync(cli: &Cli) -> std::result::Result<(), Box<dyn std::error::Error>> {
    Status::header("Env File Sync");

    let existing = env_file::parse_env_file(&cli.env_file)?;
    let missing = env_file::compute_missing_vars(&existing);

    if missing.is_empty() {
        Status::success("All known env vars are present");
        return Ok(());
    }

    if cli.dry_run {
        Status::info(&format!(
            "Would append {} missing variables (use without --dry-run to apply)",
            missing.len()
        ));
        for (key, default) in &missing {
            println!("  {} {key}={default}", "+".green());
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
    Status::info(&format!(
        "{} variables would be added",
        missing.len()
    ));

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
    Status::header("Full Migration");

    Status::step(1, 3, "Syncing vault secrets...");
    cmd_vault_sync(cli)?;

    Status::step(2, 3, "Syncing env file...");
    cmd_env_sync(cli)?;

    Status::step(3, 3, "Verifying vault functions...");
    cmd_vault_verify(cli)?;

    println!();
    Status::success("Migration complete. Run `docker compose restart functions` to apply.");

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
    fn run_vault_secret_check(&self, secret_name: &str) -> std::result::Result<bool, Box<dyn std::error::Error>>;
}

impl VaultClientExt for VaultClient {
    fn run_vault_secret_check(&self, secret_name: &str) -> std::result::Result<bool, Box<dyn std::error::Error>> {
        // Validate the secret name (alphanumeric + underscore only)
        if !secret_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(Box::new(foodshare_migrate::error::MigrateError::verification(
                format!("invalid secret name: {secret_name}"),
            )));
        }

        let exists = self.secret_exists(secret_name)?;
        Ok(exists)
    }
}
