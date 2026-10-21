# foodshare-i18n

Enterprise-grade translation management CLI for Foodshare.

## Features

- 🏥 **Health Checks** - Monitor all translation endpoints
- 🧪 **Testing** - Test translation fetch, delta sync, and ETag caching
- 🔍 **Audit** - Check translation coverage across all locales
- 🌐 **Auto-Translate** - Translate missing keys using AI
- 🚀 **Deploy** - Apply the migrations, deploy the edge functions, test the endpoints
- ♻️ **Update and backfill** - Update a locale, or backfill the posts
- 📱 **Info.plist** - Generate the localized `InfoPlist.strings` files for iOS
- ⚡ **Benchmark** - Performance testing for endpoints
- 🌍 **Locales** - List all supported languages

## Installation

```bash
# Build from source
cargo build -p foodshare-i18n --release

# Install globally
cargo install --path bins/foodshare-i18n
```

## Usage

```bash
# Show help
foodshare-i18n --help

# Health check all endpoints
foodshare-i18n health
foodshare-i18n health --detailed

# Test translation fetch for a locale
foodshare-i18n test fetch --locale en --delta --cache

# Audit translation coverage
foodshare-i18n audit                    # All locales
foodshare-i18n audit --locale de --missing   # Single locale with missing keys

# Auto-translate missing keys (dry-run)
foodshare-i18n translate --locale de

# Auto-translate and apply
foodshare-i18n translate --locale de --apply

# Translate every locale at once
foodshare-i18n translate --all
foodshare-i18n translate --all --apply

# Deploy the translation system
foodshare-i18n deploy
foodshare-i18n deploy --no-migrations --no-functions --no-test

# Update one locale
foodshare-i18n update --locale de --file messages/de.json

# Backfill the posts
foodshare-i18n backfill --batch-size 50

# Generate the localized Info.plist files
foodshare-i18n generate-infoplist

# Benchmark endpoints
foodshare-i18n bench --count 10 --locale en

# List supported locales
foodshare-i18n locales
```

## Output Formats

```bash
# Text output (default)
foodshare-i18n health

# JSON output (for scripting)
foodshare-i18n --format json health
```

In JSON mode the answer is the only thing on stdout, so a caller can
parse it; the messages go to stderr.

## Supported Locales

| Code | Language | Native Name | RTL |
|------|----------|-------------|-----|
| en | English | English | No |
| cs | Czech | Čeština | No |
| de | German | Deutsch | No |
| es | Spanish | Español | No |
| fr | French | Français | No |
| pt | Portuguese | Português | No |
| ru | Russian | Русский | No |
| uk | Ukrainian | Українська | No |
| zh | Chinese | 中文 | No |
| hi | Hindi | हिन्दी | No |
| ar | Arabic | العربية | Yes |
| it | Italian | Italiano | No |
| pl | Polish | Polski | No |
| nl | Dutch | Nederlands | No |
| ja | Japanese | 日本語 | No |
| ko | Korean | 한국어 | No |
| tr | Turkish | Türkçe | No |
| vi | Vietnamese | Tiếng Việt | No |
| id | Indonesian | Bahasa Indonesia | No |
| th | Thai | ไทย | No |
| sv | Swedish | Svenska | No |

## Backend Endpoints

The CLI communicates with these Supabase Edge Functions:

- `bff/translations` - BFF translations endpoint (recommended)
- `get-translations` - Direct translations endpoint
- `get-translations/health` - Health check
- `get-translations/locales` - Supported locales
- `get-translations/delta` - Delta sync
- `translation-audit` - Coverage audit
- `translate-batch` - AI translation

## CI/CD Integration

```yaml
# GitHub Actions example
- name: Check translation health
  run: |
    cargo run -p foodshare-i18n -- health
    
- name: Audit coverage
  run: |
    cargo run -p foodshare-i18n -- --format json audit > coverage.json
```

## Development

```bash
# Run with verbose logging
foodshare-i18n -v health

# Run tests
cargo test -p foodshare-i18n
```

## License

MIT
