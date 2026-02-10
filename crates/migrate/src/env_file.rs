//! `.env.functions` file parsing and manipulation
//!
//! Provides utilities for reading KEY=VALUE env files and appending
//! missing variables with default placeholder values.

use crate::error::{MigrateError, Result};
use std::collections::HashMap;
use std::path::Path;

/// Parse a `.env` file into a map of KEY -> VALUE.
///
/// Skips empty lines, comments (`#`), and lines without `=`.
/// Values are trimmed but not unquoted (quotes are preserved).
pub fn parse_env_file(path: &Path) -> Result<HashMap<String, String>> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        MigrateError::env_file_at(format!("failed to read: {e}"), path)
    })?;

    let mut vars = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            if !key.is_empty() {
                vars.insert(key.to_string(), value.trim().to_string());
            }
        }
    }

    Ok(vars)
}

/// Result of appending missing vars to an env file.
#[derive(Debug)]
pub struct AppendResult {
    /// Number of variables that were appended
    pub appended: usize,
    /// Number of variables that were already present
    pub skipped: usize,
}

/// Append missing variables to an env file.
///
/// Only appends vars whose keys are not already present in the file.
/// Each appended variable is written as `KEY=VALUE` on its own line.
pub fn append_missing_vars(path: &Path, vars: &[(String, String)]) -> Result<AppendResult> {
    let existing = parse_env_file(path)?;
    let mut to_append = Vec::new();
    let mut skipped = 0;

    for (key, value) in vars {
        if existing.contains_key(key) {
            skipped += 1;
        } else {
            to_append.push((key.as_str(), value.as_str()));
        }
    }

    if !to_append.is_empty() {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .map_err(|e| MigrateError::env_file_at(format!("failed to open for append: {e}"), path))?;

        writeln!(file)?;
        writeln!(file, "# Added by foodshare-migrate")?;
        for (key, value) in &to_append {
            writeln!(file, "{key}={value}")?;
        }
    }

    Ok(AppendResult {
        appended: to_append.len(),
        skipped,
    })
}

/// A group of related environment variables.
#[derive(Debug, Clone)]
pub struct EnvVarGroup {
    /// Section name (e.g. "whatsapp", "push")
    pub section: &'static str,
    /// Variables in this group: (key, default_value)
    pub vars: &'static [(&'static str, &'static str)],
}

/// Returns the groups of env vars that may be missing from `.env.functions`.
///
/// Each var has a placeholder default. After `env sync`, the user must
/// replace placeholders with real values and restart functions.
pub fn missing_env_var_groups() -> Vec<EnvVarGroup> {
    vec![
        EnvVarGroup {
            section: "whatsapp",
            vars: &[
                ("meta_webhook_verify_token", "your-meta-webhook-verify-token"),
                ("FACEBOOK_APP_ID", "your-facebook-app-id"),
                ("WHATSAPP_ACCESS_TOKEN", "your-whatsapp-access-token"),
                ("WHATSAPP_APP_SECRET", "your-whatsapp-app-secret"),
                ("WHATSAPP_PHONE_NUMBER_ID", "your-whatsapp-phone-number-id"),
                ("WHATSAPP_VERIFY_TOKEN", "your-whatsapp-verify-token"),
                ("WHATSAPP_BUSINESS_ACCOUNT_ID", ""),
            ],
        },
        EnvVarGroup {
            section: "push",
            vars: &[
                ("APNS_KEY_ID", ""),
                ("APNS_PRIVATE_KEY", ""),
                ("APNS_TEAM_ID", ""),
                ("APNS_BUNDLE_ID", "com.flutterflow.foodshare"),
                ("APNS_ENVIRONMENT", "production"),
                ("FCM_PROJECT_ID", ""),
                ("FCM_CLIENT_EMAIL", ""),
                ("FCM_PRIVATE_KEY", ""),
                ("VAPID_PUBLIC_KEY", ""),
                ("VAPID_PRIVATE_KEY", ""),
                ("VAPID_SUBJECT", "mailto:support@foodshare.club"),
            ],
        },
        EnvVarGroup {
            section: "translation",
            vars: &[
                ("DEEPL_API_KEY", ""),
                ("LLM_TRANSLATION_API_KEY", ""),
                ("LLM_TRANSLATION_ENDPOINT", ""),
                ("GOOGLE_TRANSLATE_API_KEY", ""),
                ("MICROSOFT_TRANSLATOR_API_KEY", ""),
            ],
        },
        EnvVarGroup {
            section: "app_config",
            vars: &[
                ("ENVIRONMENT", "production"),
                ("APP_URL", "https://foodshare.club"),
                ("ANDROID_PACKAGE_NAME", "com.flutterflow.foodshare"),
                ("APP_BUNDLE_ID", "com.flutterflow.foodshare"),
                ("APPLE_TEAM_ID", ""),
                ("APNS_WEBHOOK_SECRET", ""),
            ],
        },
        EnvVarGroup {
            section: "webhooks",
            vars: &[
                ("RESEND_WEBHOOK_SECRET", ""),
                ("BREVO_WEBHOOK_SECRET", ""),
                ("MAILERSEND_WEBHOOK_SECRET", ""),
                ("AWS_SES_WEBHOOK_SECRET", ""),
                ("STRIPE_WEBHOOK_SECRET", ""),
            ],
        },
        EnvVarGroup {
            section: "alerts",
            vars: &[
                ("SLACK_ALERT_WEBHOOK_URL", ""),
                ("ERROR_ALERT_WEBHOOK_URL", ""),
                ("PAGERDUTY_ROUTING_KEY", ""),
            ],
        },
    ]
}

/// Compute which env vars from the known groups are missing from the given file.
///
/// Returns `(key, default_value)` pairs for variables not present.
pub fn compute_missing_vars(
    existing: &HashMap<String, String>,
) -> Vec<(String, String)> {
    let groups = missing_env_var_groups();
    let mut missing = Vec::new();

    for group in &groups {
        for &(key, default) in group.vars {
            if !existing.contains_key(key) {
                missing.push((key.to_string(), default.to_string()));
            }
        }
    }

    missing
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_env_file_basic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env.test");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "# comment").unwrap();
            writeln!(f, "").unwrap();
            writeln!(f, "FOO=bar").unwrap();
            writeln!(f, "BAZ=qux=extra").unwrap();
            writeln!(f, "EMPTY=").unwrap();
        }
        let vars = parse_env_file(&path).unwrap();
        assert_eq!(vars.get("FOO").unwrap(), "bar");
        assert_eq!(vars.get("BAZ").unwrap(), "qux=extra");
        assert_eq!(vars.get("EMPTY").unwrap(), "");
        assert!(!vars.contains_key("# comment"));
    }

    #[test]
    fn parse_env_file_not_found() {
        let result = parse_env_file(Path::new("/nonexistent/.env"));
        assert!(result.is_err());
    }

    #[test]
    fn append_missing_vars_skips_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env.test");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "EXISTING=value").unwrap();
        }

        let vars = vec![
            ("EXISTING".to_string(), "new_value".to_string()),
            ("NEW_VAR".to_string(), "hello".to_string()),
        ];

        let result = append_missing_vars(&path, &vars).unwrap();
        assert_eq!(result.appended, 1);
        assert_eq!(result.skipped, 1);

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("NEW_VAR=hello"));
        // Existing value should not be overwritten
        assert!(content.contains("EXISTING=value"));
        assert!(!content.contains("EXISTING=new_value"));
    }

    #[test]
    fn missing_env_var_groups_has_all_sections() {
        let groups = missing_env_var_groups();
        let sections: Vec<_> = groups.iter().map(|g| g.section).collect();
        assert!(sections.contains(&"whatsapp"));
        assert!(sections.contains(&"push"));
        assert!(sections.contains(&"translation"));
        assert!(sections.contains(&"app_config"));
        assert!(sections.contains(&"webhooks"));
        assert!(sections.contains(&"alerts"));
    }
}
