//! Next.js/React security scanning
//!
//! OWASP-based security checks for Next.js applications.

use foodshare_core::error::exit_codes;
use once_cell::sync::Lazy;
use owo_colors::OwoColorize;
use regex::Regex;
use std::path::Path;

/// Severity level of a security finding
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// General information about the application's security posture
    Info,
    /// Low impact/probability issue
    Low,
    /// Moderate impact/probability issue
    Medium,
    /// Significant security risk
    High,
    /// Urgent security vulnerability that must be addressed immediately
    Critical,
}

/// OWASP Top 10 category for the security finding
#[derive(Debug, Clone)]
pub enum OwaspCategory {
    /// Broken Access Control
    A01BrokenAccessControl,
    /// Cryptographic Failures
    A02CryptographicFailures,
    /// Injection
    A03Injection,
    /// Insecure Design
    A04InsecureDesign,
    /// Security Misconfiguration
    A05SecurityMisconfiguration,
    /// Vulnerable and Outdated Components
    A06VulnerableComponents,
    /// Identification and Authentication Failures
    A07IdentificationFailures,
    /// Software and Data Integrity Failures
    A08SoftwareIntegrity,
    /// Security Logging and Monitoring Failures
    A09SecurityLogging,
    /// Server-Side Request Forgery (SSRF)
    A10Ssrf,
}

impl OwaspCategory {
    /// Get the OWASP category code (e.g., "A01:2021")
    pub fn code(&self) -> &'static str {
        match self {
            Self::A01BrokenAccessControl => "A01:2021",
            Self::A02CryptographicFailures => "A02:2021",
            Self::A03Injection => "A03:2021",
            Self::A04InsecureDesign => "A04:2021",
            Self::A05SecurityMisconfiguration => "A05:2021",
            Self::A06VulnerableComponents => "A06:2021",
            Self::A07IdentificationFailures => "A07:2021",
            Self::A08SoftwareIntegrity => "A08:2021",
            Self::A09SecurityLogging => "A09:2021",
            Self::A10Ssrf => "A10:2021",
        }
    }
}

/// A specific security finding found during scanning
#[derive(Debug)]
pub struct SecurityFinding {
    /// Path to the file containing the finding
    pub file: String,
    /// Line number of the finding
    pub line: usize,
    /// Severity level of the finding
    pub severity: Severity,
    /// OWASP category of the finding
    pub category: OwaspCategory,
    /// Description of the security issue
    pub message: String,
    /// The specific text that matched the security pattern
    pub matched_text: String,
}

/// Security pattern
#[allow(dead_code)]
struct SecurityPattern {
    name: &'static str,
    pattern: Regex,
    severity: Severity,
    category: OwaspCategory,
    message: &'static str,
}

/// Built-in security patterns
static PATTERNS: Lazy<Vec<SecurityPattern>> = Lazy::new(|| {
    vec![
        SecurityPattern {
            name: "dangerouslySetInnerHTML",
            pattern: Regex::new(r"dangerouslySetInnerHTML").unwrap(),
            severity: Severity::High,
            category: OwaspCategory::A07IdentificationFailures,
            message: "Potential XSS vulnerability",
        },
        SecurityPattern {
            name: "eval",
            pattern: Regex::new(r"\beval\s*\(").unwrap(),
            severity: Severity::Critical,
            category: OwaspCategory::A03Injection,
            message: "Code injection risk - eval() usage",
        },
        SecurityPattern {
            name: "innerHTML",
            pattern: Regex::new(r"\.innerHTML\s*=").unwrap(),
            severity: Severity::High,
            category: OwaspCategory::A03Injection,
            message: "Potential XSS - direct innerHTML assignment",
        },
    ]
});

/// Scan a file for security issues
pub fn scan_file(path: &Path) -> anyhow::Result<Vec<SecurityFinding>> {
    let content = std::fs::read_to_string(path)?;
    let file_str = path.to_string_lossy().to_string();

    let mut findings = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        for pattern in PATTERNS.iter() {
            if let Some(m) = pattern.pattern.find(line) {
                findings.push(SecurityFinding {
                    file: file_str.clone(),
                    line: line_num + 1,
                    severity: pattern.severity,
                    category: pattern.category.clone(),
                    message: pattern.message.to_string(),
                    matched_text: m.as_str().to_string(),
                });
            }
        }
    }

    Ok(findings)
}

/// Scan multiple files
pub fn scan_files(paths: &[std::path::PathBuf]) -> anyhow::Result<Vec<SecurityFinding>> {
    let mut all_findings = Vec::new();

    for path in paths {
        if path.is_file() {
            if let Ok(findings) = scan_file(path) {
                all_findings.extend(findings);
            }
        }
    }

    all_findings.sort_by(|a, b| b.severity.cmp(&a.severity));
    Ok(all_findings)
}

/// Print scan results
pub fn print_results(findings: &[SecurityFinding]) -> i32 {
    if findings.is_empty() {
        println!("{} No security issues detected", "OK".green());
        return exit_codes::SUCCESS;
    }

    let critical = findings
        .iter()
        .filter(|f| f.severity == Severity::Critical)
        .count();
    let high = findings
        .iter()
        .filter(|f| f.severity == Severity::High)
        .count();

    eprintln!(
        "{} Found {} security issue(s): {} critical, {} high",
        "ERROR".red(),
        findings.len(),
        critical,
        high
    );

    for finding in findings {
        let severity_str = match finding.severity {
            Severity::Critical => "CRITICAL".red().bold().to_string(),
            Severity::High => "HIGH".red().to_string(),
            Severity::Medium => "MEDIUM".yellow().to_string(),
            Severity::Low => "LOW".dimmed().to_string(),
            Severity::Info => "INFO".blue().to_string(),
        };

        eprintln!("  [{}] {}:{}", severity_str, finding.file, finding.line);
        eprintln!("    {}", finding.message);
    }

    if critical > 0 || high > 0 {
        exit_codes::FAILURE
    } else {
        exit_codes::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
    }

    #[test]
    fn test_owasp_category_code() {
        assert_eq!(OwaspCategory::A01BrokenAccessControl.code(), "A01:2021");
        assert_eq!(OwaspCategory::A03Injection.code(), "A03:2021");
    }
}
