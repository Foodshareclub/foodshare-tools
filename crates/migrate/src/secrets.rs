//! Vault secret definitions
//!
//! Defines the 12 secrets that need to be migrated from `.env.functions` to vault.

/// A secret to migrate from `.env.functions` into the Supabase vault.
#[derive(Debug, Clone)]
pub struct VaultSecretDef {
    /// Key name in `.env.functions`
    pub env_key: &'static str,
    /// Name of the secret in vault
    pub vault_name: &'static str,
    /// Human-readable description stored in vault
    pub description: &'static str,
}

/// The 12 secrets to migrate from `.env.functions` to vault.
///
/// These are used by PG functions like `get_openai_api_key()`,
/// `get_resend_api_key()`, `get_vault_secret()`, `get_secret_audited()`,
/// `list_required_secrets()`, and `api-v1-analytics` via RPC.
pub const VAULT_SECRETS: &[VaultSecretDef] = &[
    // AI
    VaultSecretDef {
        env_key: "OPENAI_API_KEY",
        vault_name: "OPENAI_API_KEY",
        description: "OpenAI API key for AI features",
    },
    // Email
    VaultSecretDef {
        env_key: "RESEND_API_KEY",
        vault_name: "RESEND_API_KEY",
        description: "Resend email API key",
    },
    // Redis
    VaultSecretDef {
        env_key: "UPSTASH_REDIS_TOKEN",
        vault_name: "UPSTASH_REDIS_TOKEN",
        description: "Upstash Redis authentication token",
    },
    VaultSecretDef {
        env_key: "UPSTASH_REDIS_URL",
        vault_name: "UPSTASH_REDIS_URL",
        description: "Upstash Redis REST endpoint",
    },
    VaultSecretDef {
        env_key: "UPSTASH_REDIS_REST_TOKEN",
        vault_name: "UPSTASH_REDIS_REST_TOKEN",
        description: "Upstash Redis REST authentication token",
    },
    VaultSecretDef {
        env_key: "UPSTASH_REDIS_REST_URL",
        vault_name: "UPSTASH_REDIS_REST_URL",
        description: "Upstash Redis REST endpoint",
    },
    // Airtable (same env var, stored under both names)
    VaultSecretDef {
        env_key: "AIRTABLE_API_TOKEN",
        vault_name: "AIRTABLE_API_TOKEN",
        description: "Airtable API token",
    },
    VaultSecretDef {
        env_key: "AIRTABLE_API_TOKEN",
        vault_name: "AIRTABLE_API_KEY",
        description: "Airtable API key (alias of AIRTABLE_API_TOKEN)",
    },
    // RevenueCat
    VaultSecretDef {
        env_key: "REVENUECAT_SECRET_API_KEY",
        vault_name: "REVENUECAT_SECRET_API_KEY",
        description: "RevenueCat Secret API Key",
    },
    VaultSecretDef {
        env_key: "REVENUECAT_IOS_PUBLIC_KEY",
        vault_name: "REVENUECAT_IOS_PUBLIC_KEY",
        description: "RevenueCat iOS Public API Key",
    },
    VaultSecretDef {
        env_key: "REVENUECAT_ANDROID_PUBLIC_KEY",
        vault_name: "REVENUECAT_ANDROID_PUBLIC_KEY",
        description: "RevenueCat Android Public API Key",
    },
    // Analytics
    VaultSecretDef {
        env_key: "MOTHERDUCK_TOKEN",
        vault_name: "MOTHERDUCK_TOKEN",
        description: "MotherDuck token for analytics (used by api-v1-analytics)",
    },
];

/// PG functions that read from vault and should be verified after migration.
pub const VERIFY_FUNCTIONS: &[&str] = &[
    "get_openai_api_key",
    "get_resend_api_key",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_secrets_has_expected_count() {
        assert_eq!(VAULT_SECRETS.len(), 12);
    }

    #[test]
    fn airtable_alias_uses_same_env_key() {
        let token = VAULT_SECRETS.iter().find(|s| s.vault_name == "AIRTABLE_API_TOKEN").unwrap();
        let key = VAULT_SECRETS.iter().find(|s| s.vault_name == "AIRTABLE_API_KEY").unwrap();
        assert_eq!(token.env_key, key.env_key);
    }

    #[test]
    fn all_vault_names_unique() {
        let mut names: Vec<_> = VAULT_SECRETS.iter().map(|s| s.vault_name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), VAULT_SECRETS.len());
    }
}
