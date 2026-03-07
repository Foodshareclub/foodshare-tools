//! Pre-push hook - run validation checks before pushing
//!
//! Runs a series of checks with fail-fast behavior and progress display.

use foodshare_core::error::exit_codes;
use foodshare_core::process::{command_exists, run_command};
use owo_colors::OwoColorize;
use std::time::{Duration, Instant};

/// Check definition for pre-push validation
pub struct Check {
    /// Display name of the check
    pub name: &'static str,
    /// Summary of what the check does
    pub description: &'static str,
    /// Command to execute
    pub command: &'static str,
    /// Arguments to pass to the command
    pub args: Vec<&'static str>,
    /// Whether this check must pass to allow the push
    pub required: bool,
    /// Max duration before the check is timed out
    pub timeout: Duration,
}

/// Result of a single pre-push check
#[derive(Debug)]
pub struct CheckResult {
    /// Name of the check performed
    pub name: String,
    /// Whether the check passed
    pub success: bool,
    /// Duration of the check execution
    pub duration: Duration,
    /// Combined stdout and stderr output
    pub output: Option<String>,
    /// Whether the check was skipped
    pub skipped: bool,
}

/// Configuration for pre-push checks
pub struct PrePushConfig {
    /// Whether to stop immediately after the first failure
    pub fail_fast: bool,
    /// Whether to skip non-essential checks
    pub quick_mode: bool,
    /// Max duration for the entire suite of checks
    pub timeout: Duration,
    /// List of check names to explicitly skip
    pub skip_checks: Vec<String>,
}

impl Default for PrePushConfig {
    fn default() -> Self {
        Self {
            fail_fast: true,
            quick_mode: false,
            timeout: Duration::from_secs(300),
            skip_checks: Vec::new(),
        }
    }
}

/// Run pre-push checks
pub fn run_checks(checks: &[Check], config: &PrePushConfig) -> Vec<CheckResult> {
    let mut results = Vec::new();

    println!("{}", "Running pre-push checks...".bold());
    println!();

    for check in checks {
        // Skip if in skip list
        if config.skip_checks.iter().any(|s| s == check.name) {
            results.push(CheckResult {
                name: check.name.to_string(),
                success: true,
                duration: Duration::ZERO,
                output: None,
                skipped: true,
            });
            println!(
                "  {} {} {}",
                "⊘".dimmed(),
                check.name.dimmed(),
                "(skipped)".dimmed()
            );
            continue;
        }

        // Skip non-essential checks in quick mode
        if config.quick_mode && !check.required {
            results.push(CheckResult {
                name: check.name.to_string(),
                success: true,
                duration: Duration::ZERO,
                output: None,
                skipped: true,
            });
            println!(
                "  {} {} {}",
                "⊘".dimmed(),
                check.name.dimmed(),
                "(quick mode)".dimmed()
            );
            continue;
        }

        // Check if command exists
        if !command_exists(check.command) {
            results.push(CheckResult {
                name: check.name.to_string(),
                success: !check.required,
                duration: Duration::ZERO,
                output: Some(format!("Command not found: {}", check.command)),
                skipped: false,
            });

            if check.required {
                eprintln!(
                    "  {} {} - {} not found",
                    "✗".red(),
                    check.name,
                    check.command.yellow()
                );
                if config.fail_fast {
                    break;
                }
            } else {
                println!(
                    "  {} {} - {} not found {}",
                    "⊘".dimmed(),
                    check.name.dimmed(),
                    check.command,
                    "(optional)".dimmed()
                );
            }
            continue;
        }

        // Run the check
        print!("  {} {}...", "●".blue(), check.name);
        let start = Instant::now();

        let result = run_command(check.command, &check.args);
        let duration = start.elapsed();

        match result {
            Ok(cmd_result) => {
                let success = cmd_result.success;
                results.push(CheckResult {
                    name: check.name.to_string(),
                    success,
                    duration,
                    output: Some(cmd_result.combined_output()),
                    skipped: false,
                });

                // Clear the line and print result
                print!("\r");
                if success {
                    println!(
                        "  {} {} {}",
                        "✓".green(),
                        check.name,
                        format!("({:.1}s)", duration.as_secs_f32()).dimmed()
                    );
                } else {
                    eprintln!(
                        "  {} {} {}",
                        "✗".red(),
                        check.name.red(),
                        format!("({:.1}s)", duration.as_secs_f32()).dimmed()
                    );

                    if config.fail_fast {
                        break;
                    }
                }
            }
            Err(e) => {
                results.push(CheckResult {
                    name: check.name.to_string(),
                    success: false,
                    duration,
                    output: Some(e.to_string()),
                    skipped: false,
                });

                print!("\r");
                eprintln!("  {} {} - {}", "✗".red(), check.name.red(), e);

                if config.fail_fast {
                    break;
                }
            }
        }
    }

    results
}

/// Print summary of check results
pub fn print_summary(results: &[CheckResult]) -> i32 {
    println!();

    let passed = results.iter().filter(|r| r.success && !r.skipped).count();
    let failed = results.iter().filter(|r| !r.success).count();
    let skipped = results.iter().filter(|r| r.skipped).count();
    let total_time: Duration = results.iter().map(|r| r.duration).sum();

    if failed == 0 {
        println!(
            "{} All checks passed ({} passed, {} skipped) in {:.1}s",
            "✓".green().bold(),
            passed,
            skipped,
            total_time.as_secs_f32()
        );
        exit_codes::SUCCESS
    } else {
        eprintln!(
            "{} {} check(s) failed ({} passed, {} skipped)",
            "✗".red().bold(),
            failed,
            passed,
            skipped
        );

        // Show failed check details
        for result in results.iter().filter(|r| !r.success) {
            eprintln!();
            eprintln!("  {} {}:", "Failed:".red().bold(), result.name);
            if let Some(output) = &result.output {
                for line in output.lines().take(10) {
                    eprintln!("    {}", line.dimmed());
                }
            }
        }

        exit_codes::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_push_config_default() {
        let config = PrePushConfig::default();
        assert!(config.fail_fast);
        assert!(!config.quick_mode);
        assert_eq!(config.timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_check_result_skipped() {
        let result = CheckResult {
            name: "test".to_string(),
            success: true,
            duration: Duration::ZERO,
            output: None,
            skipped: true,
        };
        assert!(result.skipped);
    }
}
