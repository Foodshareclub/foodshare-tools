//! Supabase vault operations via `docker exec psql`
//!
//! Uses `process::run_command()` from `foodshare-core` to execute
//! SQL against the Supabase DB container. No direct Postgres driver needed.

use crate::error::{MigrateError, Result};
use foodshare_core::process;
use tracing::{debug, warn};

/// Client for interacting with the Supabase vault via Docker exec.
pub struct VaultClient {
    container: String,
    db_user: String,
    db_name: String,
}

impl VaultClient {
    /// Create a new vault client.
    ///
    /// - `container`: Docker container name (e.g. `supabase-db`)
    /// - `db_user`: Postgres user (e.g. `supabase_admin`)
    /// - `db_name`: Database name (e.g. `postgres`)
    pub fn new(container: &str, db_user: &str, db_name: &str) -> Self {
        Self {
            container: container.to_string(),
            db_user: db_user.to_string(),
            db_name: db_name.to_string(),
        }
    }

    /// Run a SQL query via `docker exec psql -tAc` and return stdout.
    fn run_sql(&self, sql: &str) -> Result<String> {
        debug!(sql, "executing SQL via docker exec");

        let result = process::run_command(
            "docker",
            &[
                "exec",
                &self.container,
                "psql",
                "-U",
                &self.db_user,
                "-d",
                &self.db_name,
                "-tAc",
                sql,
            ],
        )
        .map_err(|e| MigrateError::docker(format!("failed to run docker exec: {e}")))?;

        if !result.success {
            let msg = if result.stderr.is_empty() {
                format!("psql exited with code {}", result.exit_code)
            } else {
                result.stderr.trim().to_string()
            };
            return Err(MigrateError::vault(msg));
        }

        Ok(result.stdout.trim().to_string())
    }

    /// Check that Docker is available and the container is running.
    pub fn check_connection(&self) -> Result<()> {
        if !process::command_exists("docker") {
            return Err(MigrateError::docker(
                "docker is not installed or not in PATH",
            ));
        }

        let result = process::run_command(
            "docker",
            &[
                "ps",
                "--format",
                "{{.Names}}",
                "--filter",
                &format!("name=^{}$", self.container),
            ],
        )
        .map_err(|e| MigrateError::docker(format!("failed to check container: {e}")))?;

        if !result.stdout.trim().contains(&self.container) {
            return Err(MigrateError::docker(format!(
                "container '{}' is not running",
                self.container
            )));
        }

        Ok(())
    }

    /// List all secret names in the vault.
    pub fn list_secrets(&self) -> Result<Vec<String>> {
        let output = self.run_sql("SELECT name FROM vault.decrypted_secrets ORDER BY name;")?;
        if output.is_empty() {
            return Ok(Vec::new());
        }
        Ok(output.lines().map(|l| l.trim().to_string()).collect())
    }

    /// Check if a secret exists in the vault.
    pub fn secret_exists(&self, name: &str) -> Result<bool> {
        let escaped = escape_sql_string(name);
        let output = self.run_sql(&format!(
            "SELECT COUNT(*) FROM vault.secrets WHERE name = '{escaped}';"
        ))?;
        let count: i64 = output.trim().parse().unwrap_or(0);
        Ok(count > 0)
    }

    /// Create a new secret in the vault.
    ///
    /// Does NOT check for existence first — call `secret_exists()` beforehand
    /// for idempotent behavior.
    pub fn create_secret(&self, value: &str, name: &str, description: &str) -> Result<()> {
        let escaped_value = escape_sql_string(value);
        let escaped_name = escape_sql_string(name);
        let escaped_desc = escape_sql_string(description);

        self.run_sql(&format!(
            "SELECT vault.create_secret('{escaped_value}', '{escaped_name}', '{escaped_desc}');"
        ))?;

        Ok(())
    }

    /// Update an existing secret in the vault.
    pub fn update_secret(&self, value: &str, name: &str) -> Result<()> {
        let escaped_value = escape_sql_string(value);
        let escaped_name = escape_sql_string(name);

        self.run_sql(&format!(
            "DO $$
            DECLARE
              sid UUID;
            BEGIN
              SELECT id INTO sid FROM vault.secrets WHERE name = '{escaped_name}';
              PERFORM vault.update_secret(sid, new_secret := '{escaped_value}');
            END $$;"
        ))?;

        Ok(())
    }

    /// Test a PG function by calling `SELECT fn_name()` and returning the result.
    ///
    /// Returns `Some(value)` if the function returns a non-null result,
    /// `None` if the result is null or empty.
    pub fn verify_function(&self, fn_name: &str) -> Result<Option<String>> {
        // Validate function name to prevent injection (only alphanumeric + underscore)
        if !fn_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(MigrateError::verification(format!(
                "invalid function name: {fn_name}"
            )));
        }

        let output = self.run_sql(&format!("SELECT {fn_name}();"))?;

        if output.is_empty() {
            warn!(fn_name, "function returned NULL");
            Ok(None)
        } else {
            Ok(Some(output))
        }
    }
}

/// Escape a string for use in a SQL single-quoted literal.
///
/// Doubles any single quotes to prevent SQL injection.
fn escape_sql_string(s: &str) -> String {
    s.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_sql_string_no_quotes() {
        assert_eq!(escape_sql_string("hello"), "hello");
    }

    #[test]
    fn escape_sql_string_with_quotes() {
        assert_eq!(escape_sql_string("it's"), "it''s");
        assert_eq!(escape_sql_string("a''b"), "a''''b");
    }

    #[test]
    fn escape_sql_string_empty() {
        assert_eq!(escape_sql_string(""), "");
    }

    #[test]
    fn verify_function_rejects_bad_names() {
        let client = VaultClient::new("test", "user", "db");
        let result = client.verify_function("drop table; --");
        assert!(result.is_err());
    }
}
