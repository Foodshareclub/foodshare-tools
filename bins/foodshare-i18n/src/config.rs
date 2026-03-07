//! Configuration for the i18n CLI
//!
//! URLs and environment configuration are now managed by `foodshare-api-client`.
//! This module retains locale metadata for the CLI.

use foodshare_api_client::ClientConfig;

/// Get the base URL from configuration
///
/// Delegates to `foodshare-api-client` for environment-based URL resolution.
#[must_use]
pub fn base_url() -> String {
    ClientConfig::from_env()
        .map(|c| c.base_url)
        .unwrap_or_else(|_| "https://api.foodshare.club/functions/v1".to_string())
}

/// Get the BFF URL from configuration
///
/// Delegates to `foodshare-api-client` for environment-based URL resolution.
#[must_use]
pub fn _bff_url() -> String {
    ClientConfig::from_env()
        .map(|c| c.bff_url)
        .unwrap_or_else(|_| "https://api.foodshare.club/functions/v1/bff".to_string())
}

/// Supported locales
pub const SUPPORTED_LOCALES: &[&str] = &[
    "cs", "de", "es", "fr", "pt", "ru", "uk", "zh", "hi", "ar", "it", "pl", "nl", "ja", "ko", "tr",
    "vi", "id", "th", "sv",
];

/// Locale metadata
#[derive(Debug, Clone)]
pub struct LocaleInfo {
    pub code: &'static str,
    pub name: &'static str,
    pub native_name: &'static str,
    pub flag: &'static str,
    pub rtl: bool,
}

/// Get all locale metadata
pub fn get_locale_info() -> Vec<LocaleInfo> {
    vec![
        LocaleInfo {
            code: "en",
            name: "English",
            native_name: "English",
            flag: "🇬🇧",
            rtl: false,
        },
        LocaleInfo {
            code: "cs",
            name: "Czech",
            native_name: "Čeština",
            flag: "🇨🇿",
            rtl: false,
        },
        LocaleInfo {
            code: "de",
            name: "German",
            native_name: "Deutsch",
            flag: "🇩🇪",
            rtl: false,
        },
        LocaleInfo {
            code: "es",
            name: "Spanish",
            native_name: "Español",
            flag: "🇪🇸",
            rtl: false,
        },
        LocaleInfo {
            code: "fr",
            name: "French",
            native_name: "Français",
            flag: "🇫🇷",
            rtl: false,
        },
        LocaleInfo {
            code: "pt",
            name: "Portuguese",
            native_name: "Português",
            flag: "🇵🇹",
            rtl: false,
        },
        LocaleInfo {
            code: "ru",
            name: "Russian",
            native_name: "Русский",
            flag: "🇷🇺",
            rtl: false,
        },
        LocaleInfo {
            code: "uk",
            name: "Ukrainian",
            native_name: "Українська",
            flag: "🇺🇦",
            rtl: false,
        },
        LocaleInfo {
            code: "zh",
            name: "Chinese",
            native_name: "中文",
            flag: "🇨🇳",
            rtl: false,
        },
        LocaleInfo {
            code: "hi",
            name: "Hindi",
            native_name: "हिन्दी",
            flag: "🇮🇳",
            rtl: false,
        },
        LocaleInfo {
            code: "ar",
            name: "Arabic",
            native_name: "العربية",
            flag: "🇸🇦",
            rtl: true,
        },
        LocaleInfo {
            code: "it",
            name: "Italian",
            native_name: "Italiano",
            flag: "🇮🇹",
            rtl: false,
        },
        LocaleInfo {
            code: "pl",
            name: "Polish",
            native_name: "Polski",
            flag: "🇵🇱",
            rtl: false,
        },
        LocaleInfo {
            code: "nl",
            name: "Dutch",
            native_name: "Nederlands",
            flag: "🇳🇱",
            rtl: false,
        },
        LocaleInfo {
            code: "ja",
            name: "Japanese",
            native_name: "日本語",
            flag: "🇯🇵",
            rtl: false,
        },
        LocaleInfo {
            code: "ko",
            name: "Korean",
            native_name: "한국어",
            flag: "🇰🇷",
            rtl: false,
        },
        LocaleInfo {
            code: "tr",
            name: "Turkish",
            native_name: "Türkçe",
            flag: "🇹🇷",
            rtl: false,
        },
        LocaleInfo {
            code: "vi",
            name: "Vietnamese",
            native_name: "Tiếng Việt",
            flag: "🇻🇳",
            rtl: false,
        },
        LocaleInfo {
            code: "id",
            name: "Indonesian",
            native_name: "Bahasa Indonesia",
            flag: "🇮🇩",
            rtl: false,
        },
        LocaleInfo {
            code: "th",
            name: "Thai",
            native_name: "ไทย",
            flag: "🇹🇭",
            rtl: false,
        },
        LocaleInfo {
            code: "sv",
            name: "Swedish",
            native_name: "Svenska",
            flag: "🇸🇪",
            rtl: false,
        },
    ]
}
