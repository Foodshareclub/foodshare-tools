//! Supabase migrations status checking
//!
//! Validates that migrations are properly committed and staged.

use foodshare_core::error::exit_codes;
use foodshare_core::git::GitRepo;
use owo_colors::OwoColorize;
use std::path::Path;
use walkdir::WalkDir;

/// Migration file info
#[derive(Debug)]
pub struct MigrationFile {
    pub path: String,
    pub name: String,
    pub timestamp: String,
}

/// Check migrations status
pub struct MigrationsCheck {
    pub migrations_dir: String,
    pub uncommitted: Vec<MigrationFile>,
    pub staged: Vec<MigrationFile>,
    pub total: usize,
}

/// Check for migration files in a directory
pub fn check_migrations(
    migrations_dir: &Path,
    check_uncommitted: bool,
    check_staged: bool,
) -> anyhow::Result<MigrationsCheck> {
    let mut result = MigrationsCheck {
        migrations_dir: migrations_dir.to_string_lossy().to_string(),
        uncommitted: Vec::new(),
        staged: Vec::new(),
        total: 0,
    };

    // Find all migration files
    let migration_files: Vec<_> = WalkDir::new(migrations_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .map_or(false, |ext| ext == "sql")
        })
        .collect();

    result.total = migration_files.len();

    if !check_uncommitted && !check_staged {
        return Ok(result);
    }

    // Get git status
    let repo = GitRepo::open_current()?;

    if check_uncommitted {
        let uncommitted = repo.uncommitted_files()?;
        for entry in &migration_files {
            let path = entry.path();
            if uncommitted.iter().any(|u| path.ends_with(u)) {
                if let Some(migration) = parse_migration_file(path) {
                    result.uncommitted.push(migration);
                }
            }
        }
    }

    if check_staged {
        let staged = repo.staged_files()?;
        for entry in &migration_files {
            let path = entry.path();
            if staged.iter().any(|s| path.ends_with(s)) {
                if let Some(migration) = parse_migration_file(path) {
                    result.staged.push(migration);
                }
            }
        }
    }

    Ok(result)
}

/// Parse migration file name to extract info
fn parse_migration_file(path: &Path) -> Option<MigrationFile> {
    let file_name = path.file_name()?.to_string_lossy().to_string();

    // Migration format: YYYYMMDDHHMMSS_name.sql
    let parts: Vec<&str> = file_name.splitn(2, '_').collect();
    let timestamp = parts.first().unwrap_or(&"").to_string();
    let name = parts
        .get(1)
        .map(|s| s.trim_end_matches(".sql"))
        .unwrap_or("")
        .to_string();

    Some(MigrationFile {
        path: path.to_string_lossy().to_string(),
        name,
        timestamp,
    })
}

/// Print migrations check results
pub fn print_results(check: &MigrationsCheck) -> i32 {
    println!(
        "{} Found {} migration(s) in {}",
        "ℹ".blue(),
        check.total,
        check.migrations_dir
    );

    let mut has_issues = false;

    if !check.uncommitted.is_empty() {
        has_issues = true;
        eprintln!();
        eprintln!(
            "{} {} uncommitted migration(s):",
            "⚠".yellow(),
            check.uncommitted.len()
        );
        for m in &check.uncommitted {
            eprintln!("  - {} ({})", m.name.yellow(), m.timestamp.dimmed());
        }
    }

    if !check.staged.is_empty() {
        println!();
        println!(
            "{} {} staged migration(s):",
            "✓".green(),
            check.staged.len()
        );
        for m in &check.staged {
            println!("  - {} ({})", m.name.green(), m.timestamp.dimmed());
        }
    }

    if has_issues {
        exit_codes::FAILURE
    } else {
        exit_codes::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_migration_file() {
        let path = PathBuf::from("supabase/migrations/20240101120000_create_users.sql");
        let migration = parse_migration_file(&path).unwrap();

        assert_eq!(migration.timestamp, "20240101120000");
        assert_eq!(migration.name, "create_users");
    }

    #[test]
    fn test_parse_migration_file_no_underscore() {
        let path = PathBuf::from("supabase/migrations/20240101120000.sql");
        let migration = parse_migration_file(&path).unwrap();

        assert_eq!(migration.timestamp, "20240101120000.sql");
    }
}
