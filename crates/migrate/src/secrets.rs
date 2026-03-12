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
    // Core Infrastructure
    VaultSecretDef {
        env_key: "JWT_SECRET",
        vault_name: "JWT_SECRET",
        description: "Secret key for JWT generation and verification",
    },
    VaultSecretDef {
        env_key: "CHAT_ENCRYPTION_KEY",
        vault_name: "CHAT_ENCRYPTION_KEY",
        description: "Key for encrypting and decrypting chat messages",
    },
    VaultSecretDef {
        env_key: "CRON_SECRET",
        vault_name: "CRON_SECRET",
        description: "Secret for authorizing cron job triggers",
    },
    // Bootstrapping
    VaultSecretDef {
        env_key: "POSTGRES_PASSWORD",
        vault_name: "POSTGRES_PASSWORD",
        description: "Postgres root password",
    },
    VaultSecretDef {
        env_key: "ANON_KEY",
        vault_name: "ANON_KEY",
        description: "Supabase anonymous API key",
    },
    VaultSecretDef {
        env_key: "SERVICE_ROLE_KEY",
        vault_name: "SERVICE_ROLE_KEY",
        description: "Supabase service_role API key",
    },
    // Monitoring & Alerts
    VaultSecretDef {
        env_key: "SLACK_WEBHOOK_URL",
        vault_name: "SLACK_WEBHOOK_URL",
        description: "Slack webhook URL for alerts and notifications",
    },
    VaultSecretDef {
        env_key: "ERROR_ALERT_WEBHOOK_URL",
        vault_name: "ERROR_ALERT_WEBHOOK_URL",
        description: "Slack webhook URL for error reporting",
    },
    VaultSecretDef {
        env_key: "PAGERDUTY_ROUTING_KEY",
        vault_name: "PAGERDUTY_ROUTING_KEY",
        description: "PagerDuty routing key for incident alerting",
    },
    // External Services - Vector & Search
    VaultSecretDef {
        env_key: "UPSTASH_VECTOR_REST_URL",
        vault_name: "UPSTASH_VECTOR_REST_URL",
        description: "Upstash Vector REST endpoint",
    },
    VaultSecretDef {
        env_key: "UPSTASH_VECTOR_REST_TOKEN",
        vault_name: "UPSTASH_VECTOR_REST_TOKEN",
        description: "Upstash Vector REST authentication token",
    },
    VaultSecretDef {
        env_key: "UPSTASH_SEARCH_REST_URL",
        vault_name: "UPSTASH_SEARCH_REST_URL",
        description: "Upstash Search REST endpoint",
    },
    VaultSecretDef {
        env_key: "UPSTASH_SEARCH_REST_TOKEN",
        vault_name: "UPSTASH_SEARCH_REST_TOKEN",
        description: "Upstash Search REST authentication token",
    },
    VaultSecretDef {
        env_key: "QSTASH_URL",
        vault_name: "QSTASH_URL",
        description: "QStash messaging endpoint",
    },
    VaultSecretDef {
        env_key: "QSTASH_TOKEN",
        vault_name: "QSTASH_TOKEN",
        description: "QStash authentication token",
    },
    // Additional Email Providers
    VaultSecretDef {
        env_key: "BREVO_API_KEY",
        vault_name: "BREVO_API_KEY",
        description: "Brevo (Sendinblue) email API key",
    },
    VaultSecretDef {
        env_key: "MAILERSEND_API_KEY",
        vault_name: "MAILERSEND_API_KEY",
        description: "MailerSend email API key",
    },
    // Image Processing
    VaultSecretDef {
        env_key: "TINYPNG_API_KEY",
        vault_name: "TINYPNG_API_KEY",
        description: "TinyPNG API key for image compression",
    },
    VaultSecretDef {
        env_key: "CLOUDINARY_CLOUD_NAME",
        vault_name: "CLOUDINARY_CLOUD_NAME",
        description: "Cloudinary cloud name",
    },
    VaultSecretDef {
        env_key: "CLOUDINARY_API_KEY",
        vault_name: "CLOUDINARY_API_KEY",
        description: "Cloudinary API key",
    },
    VaultSecretDef {
        env_key: "CLOUDINARY_API_SECRET",
        vault_name: "CLOUDINARY_API_SECRET",
        description: "Cloudinary API secret",
    },
    // AI - HuggingFace & Others
    VaultSecretDef {
        env_key: "HUGGINGFACE_TOKEN",
        vault_name: "HUGGINGFACE_TOKEN",
        description: "HuggingFace API token for AI models",
    },
    VaultSecretDef {
        env_key: "GROQ_API_KEY",
        vault_name: "GROQ_API_KEY",
        description: "Groq API key for AI features",
    },
    VaultSecretDef {
        env_key: "AI_GATEWAY_API_KEY",
        vault_name: "AI_GATEWAY_API_KEY",
        description: "AI Gateway API key",
    },
    // Comms - WhatsApp, Telegram, Twilio
    VaultSecretDef {
        env_key: "WHATSAPP_APP_SECRET",
        vault_name: "WHATSAPP_APP_SECRET",
        description: "WhatsApp/Meta app secret for webhook verification",
    },
    VaultSecretDef {
        env_key: "BOT_TOKEN",
        vault_name: "BOT_TOKEN",
        description: "Telegram bot token",
    },
    VaultSecretDef {
        env_key: "ADMIN_CHAT_ID",
        vault_name: "ADMIN_CHAT_ID",
        description: "Telegram admin chat ID for alerts",
    },
    VaultSecretDef {
        env_key: "TWILIO_ACCOUNT_SID",
        vault_name: "TWILIO_ACCOUNT_SID",
        description: "Twilio Account SID",
    },
    VaultSecretDef {
        env_key: "TWILIO_AUTH_TOKEN",
        vault_name: "TWILIO_AUTH_TOKEN",
        description: "Twilio Auth Token",
    },
    VaultSecretDef {
        env_key: "TWILIO_VERIFY_SERVICE_SID",
        vault_name: "TWILIO_VERIFY_SERVICE_SID",
        description: "Twilio Verify Service SID",
    },
    // Storage - Cloudflare R2
    VaultSecretDef {
        env_key: "R2_ACCOUNT_ID",
        vault_name: "R2_ACCOUNT_ID",
        description: "Cloudflare R2 Account ID",
    },
    VaultSecretDef {
        env_key: "R2_ACCESS_KEY_ID",
        vault_name: "R2_ACCESS_KEY_ID",
        description: "Cloudflare R2 Access Key ID",
    },
    VaultSecretDef {
        env_key: "R2_SECRET_ACCESS_KEY",
        vault_name: "R2_SECRET_ACCESS_KEY",
        description: "Cloudflare R2 Secret Access Key",
    },
    VaultSecretDef {
        env_key: "R2_BUCKET_NAME",
        vault_name: "R2_BUCKET_NAME",
        description: "Cloudflare R2 Bucket Name",
    },
    VaultSecretDef {
        env_key: "R2_PUBLIC_URL",
        vault_name: "R2_PUBLIC_URL",
        description: "Cloudflare R2 Public URL",
    },
    // AWS - General / SES
    VaultSecretDef {
        env_key: "AWS_ACCESS_KEY_ID",
        vault_name: "AWS_ACCESS_KEY_ID",
        description: "AWS Access Key ID",
    },
    VaultSecretDef {
        env_key: "AWS_SECRET_ACCESS_KEY",
        vault_name: "AWS_SECRET_ACCESS_KEY",
        description: "AWS Secret Access Key",
    },
    // Translation
    VaultSecretDef {
        env_key: "DEEPL_API_KEY",
        vault_name: "DEEPL_API_KEY",
        description: "DeepL API key for translations",
    },
     VaultSecretDef {
        env_key: "GOOGLE_TRANSLATE_API_KEY",
        vault_name: "GOOGLE_TRANSLATE_API_KEY",
        description: "Google Translate API key",
    },
    // Webhooks & Payments
    VaultSecretDef {
        env_key: "STRIPE_WEBHOOK_SECRET",
        vault_name: "STRIPE_WEBHOOK_SECRET",
        description: "Stripe webhook signing secret",
    },
    // Apple Auth
    VaultSecretDef {
        env_key: "GOTRUE_EXTERNAL_APPLE_CLIENT_ID",
        vault_name: "GOTRUE_EXTERNAL_APPLE_CLIENT_ID",
        description: "Apple OAuth Client ID (Service ID)",
    },
    VaultSecretDef {
        env_key: "GOTRUE_EXTERNAL_APPLE_TEAM_ID",
        vault_name: "GOTRUE_EXTERNAL_APPLE_TEAM_ID",
        description: "Apple Team ID",
    },
    VaultSecretDef {
        env_key: "GOTRUE_EXTERNAL_APPLE_KEY_ID",
        vault_name: "GOTRUE_EXTERNAL_APPLE_KEY_ID",
        description: "Apple Key ID",
    },
    VaultSecretDef {
        env_key: "GOTRUE_EXTERNAL_APPLE_PRIVATE_KEY",
        vault_name: "GOTRUE_EXTERNAL_APPLE_PRIVATE_KEY",
        description: "Apple Private Key (.p8 file content)",
    },
    // Google Auth
    VaultSecretDef {
        env_key: "GOTRUE_EXTERNAL_GOOGLE_CLIENT_ID",
        vault_name: "GOTRUE_EXTERNAL_GOOGLE_CLIENT_ID",
        description: "Google OAuth Client ID",
    },
    VaultSecretDef {
        env_key: "GOTRUE_EXTERNAL_GOOGLE_SECRET",
        vault_name: "GOTRUE_EXTERNAL_GOOGLE_SECRET",
        description: "Google OAuth Secret",
    },
    // Facebook Auth
    VaultSecretDef {
        env_key: "GOTRUE_EXTERNAL_FACEBOOK_CLIENT_ID",
        vault_name: "GOTRUE_EXTERNAL_FACEBOOK_CLIENT_ID",
        description: "Facebook OAuth Client ID",
    },
    VaultSecretDef {
        env_key: "GOTRUE_EXTERNAL_FACEBOOK_SECRET",
        vault_name: "GOTRUE_EXTERNAL_FACEBOOK_SECRET",
        description: "Facebook OAuth Secret",
    },
];

/// PG functions that read from vault and should be verified after migration.
pub const VERIFY_FUNCTIONS: &[&str] = &["get_openai_api_key", "get_resend_api_key"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_secrets_has_expected_count() {
        assert_eq!(VAULT_SECRETS.len(), 60);
    }

    #[test]
    fn airtable_alias_uses_same_env_key() {
        let token = VAULT_SECRETS
            .iter()
            .find(|s| s.vault_name == "AIRTABLE_API_TOKEN")
            .unwrap();
        let key = VAULT_SECRETS
            .iter()
            .find(|s| s.vault_name == "AIRTABLE_API_KEY")
            .unwrap();
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
