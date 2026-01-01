//! Secret scanning - detect sensitive data in staged files
//!
//! Scans files for potential secrets, API keys, passwords, and other sensitive data.

use foodshare_core::config::SecretsConfig;
use foodshare_core::error::exit_codes;
use once_cell::sync::Lazy;
use owo_colors::OwoColorize;
use regex::Regex;
use std::path::Path;

/// Secret pattern definition
struct SecretPattern {
    name: &'static str,
    pattern: Regex,
    severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    High,
    Medium,
    Low,
}

/// A detected secret
#[derive(Debug)]
pub struct SecretMatch {
    pub file: String,
    pub line: usize,
    pub pattern_name: String,
    pub matched_text: String,
    pub severity: Severity,
}

/// Built-in secret patterns
static PATTERNS: Lazy<Vec<SecretPattern>> = Lazy::new(|| {
    vec![
        // API Keys
        SecretPattern {
            name: "AWS Access Key",
            pattern: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
            severity: Severity::High,
        },
        SecretPattern {
            name: "AWS Secret Key",
            pattern: Regex::new(r#"(?i)aws[_\-]?secret[_\-]?access[_\-]?key\s*[=:]\s*["']?[A-Za-z0-9/+=]{40}"#).unwrap(),
            severity: Severity::High,
        },
        SecretPattern {
            name: "GitHub Token",
            pattern: Regex::new(r"gh[pousr]_[A-Za-z0-9_]{36,}").unwrap(),
            severity: Severity::High,
        },
        SecretPattern {
            name: "Generic API Key",
            pattern: Regex::new(r#"(?i)(api[_\-]?key|apikey)\s*[=:]\s*["']?[A-Za-z0-9_\-]{20,}"#).unwrap(),
            severity: Severity::Medium,
        },
        // Supabase
        SecretPattern {
            name: "Supabase Service Key",
            pattern: Regex::new(r"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+").unwrap(),
            severity: Severity::High,
        },
        // Private Keys
        SecretPattern {
            name: "Private Key",
            pattern: Regex::new(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap(),
            severity: Severity::High,
        },
        // Passwords
        SecretPattern {
            name: "Password Assignment",
            pattern: Regex::new(r#"(?i)(password|passwd|pwd)\s*[=:]\s*["'][^"']{8,}["']"#).unwrap(),
            severity: Severity::High,
        },
        // Database URLs
        SecretPattern {
            name: "Database URL",
            pattern: Regex::new(r"(?i)(postgres|mysql|mongodb)://[^:]+:[^@]+@").unwrap(),
            severity: Severity::High,
        },
        // Slack/Discord webhooks
        SecretPattern {
            name: "Slack Webhook",
            pattern: Regex::new(r"https://hooks\.slack\.com/services/T[A-Z0-9]+/B[A-Z0-9]+/[A-Za-z0-9]+").unwrap(),
            severity: Severity::Medium,
        },
        SecretPattern {
            name: "Discord Webhook",
            pattern: Regex::new(r"https://discord(app)?\.com/api/webhooks/\d+/[A-Za-z0-9_-]+").unwrap(),
            severity: Severity::Medium,
        },
        // Debug statements (lower severity)
        SecretPattern {
            name: "Debug Print",
            pattern: Regex::new(r#"(?i)(console\.log|print|NSLog|debugPrint)\s*\(\s*["'].*password.*["']"#).unwrap(),
            severity: Severity::Low,
        },
    ]
});

/// Scan a file for secrets
pub fn scan_file(path: &Path, config: &SecretsConfig) -> anyhow::Result<Vec<SecretMatch>> {
    let content = std::fs::read_to_string(path)?;
    let file_str = path.to_string_lossy().to_string();

    // Check if file should be excluded
    for exclude in &config.exclude_files {
        if file_str.contains(exclude) {
            return Ok(Vec::new());
        }
    }

    let mut matches = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        // Skip excluded patterns
        let should_skip = config
            .exclude_patterns
            .iter()
            .any(|p| line.contains(p));
        if should_skip {
            continue;
        }

        for pattern in PATTERNS.iter() {
            if let Some(m) = pattern.pattern.find(line) {
                matches.push(SecretMatch {
                    file: file_str.clone(),
                    line: line_num + 1,
                    pattern_name: pattern.name.to_string(),
                    matched_text: mask_secret(m.as_str()),
                    severity: pattern.severity,
                });
            }
        }
    }

    Ok(matches)
}

/// Scan multiple files for secrets
pub fn scan_files(paths: &[std::path::PathBuf], config: &SecretsConfig) -> anyhow::Result<Vec<SecretMatch>> {
    let mut all_matches = Vec::new();

    for path in paths {
        if path.is_file() {
            match scan_file(path, config) {
                Ok(matches) => all_matches.extend(matches),
                Err(e) => {
                    eprintln!("{}: Failed to scan {}: {}", "warning".yellow(), path.display(), e);
                }
            }
        }
    }

    Ok(all_matches)
}

/// Mask a secret for display (show first/last few chars)
fn mask_secret(secret: &str) -> String {
    if secret.len() <= 8 {
        "*".repeat(secret.len())
    } else {
        format!(
            "{}...{}",
            &secret[..4],
            &secret[secret.len() - 4..]
        )
    }
}

/// Print scan results
pub fn print_results(matches: &[SecretMatch]) -> i32 {
    if matches.is_empty() {
        println!("{} No secrets detected", "OK".green());
        return exit_codes::SUCCESS;
    }

    eprintln!(
        "{} Found {} potential secret(s):",
        "ERROR".red(),
        matches.len()
    );
    eprintln!();

    for m in matches {
        let severity_str = match m.severity {
            Severity::High => "HIGH".red().bold().to_string(),
            Severity::Medium => "MEDIUM".yellow().to_string(),
            Severity::Low => "LOW".dimmed().to_string(),
        };

        eprintln!(
            "  [{}] {} (line {})",
            severity_str,
            m.file,
            m.line
        );
        eprintln!("    Pattern: {}", m.pattern_name.cyan());
        eprintln!("    Match: {}", m.matched_text.dimmed());
        eprintln!();
    }

    exit_codes::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_secret_short() {
        assert_eq!(mask_secret("abc"), "***");
    }

    #[test]
    fn test_mask_secret_long() {
        let masked = mask_secret("abcdefghijklmnop");
        assert!(masked.starts_with("abcd"));
        assert!(masked.ends_with("mnop"));
        assert!(masked.contains("..."));
    }

    #[test]
    fn test_aws_key_pattern() {
        let pattern = &PATTERNS[0];
        assert!(pattern.pattern.is_match("AKIAIOSFODNN7EXAMPLE"));
        assert!(!pattern.pattern.is_match("not_an_aws_key"));
    }

    #[test]
    fn test_github_token_pattern() {
        let pattern = &PATTERNS[2];
        assert!(pattern.pattern.is_match("ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"));
        assert!(!pattern.pattern.is_match("not_a_github_token"));
    }
}
