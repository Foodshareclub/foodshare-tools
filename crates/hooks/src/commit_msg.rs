//! Commit message validation - enforce conventional commits format
//!
//! Validates commit messages against the conventional commits specification.
//! https://www.conventionalcommits.org

use foodshare_core::config::CommitMsgConfig;
use foodshare_core::error::exit_codes;
use owo_colors::OwoColorize;
use regex::Regex;
use std::fs;
use std::path::Path;

/// Validation result
pub struct ValidationResult {
    pub valid: bool,
    pub exit_code: i32,
    pub message: Option<String>,
}

/// Validate a commit message file
pub fn validate_commit_message(
    file: &Path,
    config: &CommitMsgConfig,
) -> anyhow::Result<ValidationResult> {
    let commit_msg = fs::read_to_string(file)?;
    let commit_msg = commit_msg.trim();

    if commit_msg.is_empty() {
        return Ok(ValidationResult {
            valid: false,
            exit_code: exit_codes::FAILURE,
            message: Some("Commit message is empty".to_string()),
        });
    }

    // Skip validation for merge commits
    if config.skip_merge && commit_msg.starts_with("Merge ") {
        return Ok(ValidationResult {
            valid: true,
            exit_code: exit_codes::SUCCESS,
            message: Some("Skipping validation for merge commit".to_string()),
        });
    }

    // Skip validation for revert commits
    if config.skip_revert && commit_msg.starts_with("Revert ") {
        return Ok(ValidationResult {
            valid: true,
            exit_code: exit_codes::SUCCESS,
            message: Some("Skipping validation for revert commit".to_string()),
        });
    }

    // Get the first line (subject)
    let subject = commit_msg.lines().next().unwrap_or("");

    // Build conventional commit regex
    let types_pattern = config.types.join("|");
    let pattern = format!(r"^({})(\([a-zA-Z0-9_-]+\))?!?:\s+.+$", types_pattern);
    let regex = Regex::new(&pattern)?;

    if !regex.is_match(subject) {
        return Ok(ValidationResult {
            valid: false,
            exit_code: exit_codes::FAILURE,
            message: Some(format!("Invalid commit message format: {}", subject)),
        });
    }

    // Extract the description part (after the colon)
    let description = if let Some(colon_pos) = subject.find(':') {
        subject[colon_pos + 1..].trim()
    } else {
        subject
    };

    // Check description length
    if description.len() < config.min_length {
        return Ok(ValidationResult {
            valid: false,
            exit_code: exit_codes::FAILURE,
            message: Some(format!(
                "Commit description is too short ({} chars, minimum {})",
                description.len(),
                config.min_length
            )),
        });
    }

    // Check subject length (warning only)
    if subject.len() > config.max_length {
        // This is a warning, not an error
        eprintln!(
            "{}: Subject line is long ({} chars, recommended max {})",
            "warning".yellow(),
            subject.len(),
            config.max_length
        );
    }

    // Check for capitalization (warning only)
    if let Some(first_char) = description.chars().next() {
        if first_char.is_uppercase() {
            eprintln!(
                "{}: Description should start with lowercase letter",
                "warning".yellow()
            );
        }
    }

    // Check for trailing period (warning only)
    if description.ends_with('.') {
        eprintln!(
            "{}: Description should not end with a period",
            "warning".yellow()
        );
    }

    Ok(ValidationResult {
        valid: true,
        exit_code: exit_codes::SUCCESS,
        message: Some("Commit message is valid".to_string()),
    })
}

/// Print error message with formatting
pub fn print_error(subject: &str, types: &[String]) {
    eprintln!("{}", "Invalid commit message format".red().bold());
    eprintln!();
    eprintln!("  Received: {}", subject.red());
    eprintln!();
    eprintln!("  {}", "Expected format:".bold());
    eprintln!(
        "    {}({}): {}",
        "<type>".cyan(),
        "scope".dimmed(),
        "<description>".green()
    );
    eprintln!();
    eprintln!("  {}", "Valid types:".bold());
    eprintln!("    {}", types.join(", ").cyan());
    eprintln!();
    eprintln!("  {}", "Examples:".bold());
    eprintln!("    {}", "feat(auth): add login with Apple".green());
    eprintln!("    {}", "fix(ui): resolve button alignment issue".green());
    eprintln!(
        "    {}",
        "docs: update README with setup instructions".green()
    );
    eprintln!("    {}", "chore(deps): update dependencies".green());
    eprintln!();
    eprintln!("  {}", "Breaking changes:".bold());
    eprintln!(
        "    {}",
        "feat(api)!: change authentication endpoint".yellow()
    );
    eprintln!();
    eprintln!(
        "  For more info: {}",
        "https://www.conventionalcommits.org".underline()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> CommitMsgConfig {
        CommitMsgConfig::default()
    }

    fn test_commit(msg: &str) -> bool {
        let config = default_config();
        let types_pattern = config.types.join("|");
        let pattern = format!(r"^({})(\([a-zA-Z0-9_-]+\))?!?:\s+.+$", types_pattern);
        let regex = Regex::new(&pattern).unwrap();
        regex.is_match(msg)
    }

    #[test]
    fn test_valid_commits() {
        assert!(test_commit("feat: add new feature"));
        assert!(test_commit("feat(auth): add login"));
        assert!(test_commit("fix(ui): resolve button issue"));
        assert!(test_commit("docs: update README"));
        assert!(test_commit("feat!: breaking change"));
        assert!(test_commit("feat(api)!: breaking API change"));
    }

    #[test]
    fn test_invalid_commits() {
        assert!(!test_commit("Add new feature"));
        assert!(!test_commit("feat add new feature"));
        assert!(!test_commit("feature: add new feature"));
        assert!(!test_commit("FEAT: add new feature"));
        assert!(!test_commit("feat():  add new feature"));
    }
}
