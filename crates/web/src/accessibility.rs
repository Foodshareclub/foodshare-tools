//! Accessibility (a11y) checks for JSX/TSX
//!
//! Checks React components for common accessibility issues.

use foodshare_core::error::exit_codes;
use once_cell::sync::Lazy;
use owo_colors::OwoColorize;
use regex::Regex;
use std::path::Path;

/// Accessibility issue
#[derive(Debug)]
pub struct A11yIssue {
    pub file: String,
    pub line: usize,
    pub rule: String,
    pub message: String,
    pub severity: A11ySeverity,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum A11ySeverity {
    Error,
    Warning,
}

/// A11y check pattern
struct A11yPattern {
    name: &'static str,
    pattern: Regex,
    message: &'static str,
    severity: A11ySeverity,
}

/// Built-in a11y patterns
static PATTERNS: Lazy<Vec<A11yPattern>> = Lazy::new(|| {
    vec![
        // Autofocus (simple pattern that works)
        A11yPattern {
            name: "no-autofocus",
            pattern: Regex::new(r#"autoFocus"#).unwrap(),
            message: "Avoid using autoFocus as it can cause accessibility issues",
            severity: A11ySeverity::Warning,
        },
        // Empty anchor href
        A11yPattern {
            name: "anchor-has-content",
            pattern: Regex::new(r#"<a[^>]*href=["']#["'][^>]*>\s*</a>"#).unwrap(),
            message: "Anchor with href='#' should have meaningful content",
            severity: A11ySeverity::Warning,
        },
        // tabIndex with positive value (bad practice)
        A11yPattern {
            name: "no-positive-tabindex",
            pattern: Regex::new(r#"tabIndex=\{?[1-9]"#).unwrap(),
            message: "Avoid positive tabIndex values as they disrupt natural tab order",
            severity: A11ySeverity::Warning,
        },
        // role="presentation" or role="none" on interactive elements
        A11yPattern {
            name: "no-interactive-element-to-noninteractive-role",
            pattern: Regex::new(r#"<(button|a|input)[^>]*role=["'](presentation|none)["']"#).unwrap(),
            message: "Interactive elements should not have presentation/none role",
            severity: A11ySeverity::Error,
        },
    ]
});

/// Check a file for a11y issues
pub fn check_file(path: &Path) -> anyhow::Result<Vec<A11yIssue>> {
    let content = std::fs::read_to_string(path)?;
    let file_str = path.to_string_lossy().to_string();

    // Only check JSX/TSX files
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if !["jsx", "tsx"].contains(&ext) {
        return Ok(Vec::new());
    }

    let mut issues = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        for pattern in PATTERNS.iter() {
            if pattern.pattern.is_match(line) {
                issues.push(A11yIssue {
                    file: file_str.clone(),
                    line: line_num + 1,
                    rule: pattern.name.to_string(),
                    message: pattern.message.to_string(),
                    severity: pattern.severity,
                });
            }
        }
    }

    Ok(issues)
}

/// Check multiple files
pub fn check_files(paths: &[std::path::PathBuf]) -> anyhow::Result<Vec<A11yIssue>> {
    let mut all_issues = Vec::new();

    for path in paths {
        if path.is_file() {
            match check_file(path) {
                Ok(issues) => all_issues.extend(issues),
                Err(e) => {
                    eprintln!("{}: Failed to check {}: {}", "warning".yellow(), path.display(), e);
                }
            }
        }
    }

    Ok(all_issues)
}

/// Print a11y check results
pub fn print_results(issues: &[A11yIssue]) -> i32 {
    if issues.is_empty() {
        println!("{} No accessibility issues detected", "OK".green());
        return exit_codes::SUCCESS;
    }

    let errors = issues.iter().filter(|i| i.severity == A11ySeverity::Error).count();
    let warnings = issues.iter().filter(|i| i.severity == A11ySeverity::Warning).count();

    eprintln!(
        "{} Found {} accessibility issue(s): {} errors, {} warnings",
        "ERROR".red(),
        issues.len(),
        errors,
        warnings
    );
    eprintln!();

    for issue in issues {
        let severity_str = match issue.severity {
            A11ySeverity::Error => "error".red().to_string(),
            A11ySeverity::Warning => "warning".yellow().to_string(),
        };

        eprintln!(
            "  {}:{} {} [{}]",
            issue.file,
            issue.line,
            severity_str,
            issue.rule.cyan()
        );
        eprintln!("    {}", issue.message);
        eprintln!();
    }

    if errors > 0 {
        exit_codes::FAILURE
    } else {
        exit_codes::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a11y_severity() {
        assert_ne!(A11ySeverity::Error, A11ySeverity::Warning);
    }

    #[test]
    fn test_autofocus_pattern() {
        let pattern = &PATTERNS[0];
        assert!(pattern.pattern.is_match("<input autoFocus />"));
        assert!(!pattern.pattern.is_match("<input />"));
    }

    #[test]
    fn test_positive_tabindex_pattern() {
        let pattern = &PATTERNS[2];
        assert!(pattern.pattern.is_match("tabIndex={5}"));
        assert!(!pattern.pattern.is_match("tabIndex={0}"));
        assert!(!pattern.pattern.is_match("tabIndex={-1}"));
    }
}
