# foodshare-i18n

Enterprise-grade translation management CLI for Foodshare.

## Features

- 🏥 **Health Checks** - Monitor all translation endpoints
- 📊 **Status** - View overall translation system status
- 🧪 **Testing** - Test translation fetch, delta sync, and ETag caching
- 🔍 **Audit** - Check translation coverage across all locales
- 🌐 **Auto-Translate** - Translate missing keys using AI
- 🔄 **Sync** - Sync all locales at once
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

# Check system status
foodshare-i18n status

# Health check all endpoints
foodshare-i18n health --timing

# Test translation fetch for a locale
foodshare-i18n test en --delta --cache

# Audit translation coverage
foodshare-i18n audit                    # All locales
foodshare-i18n audit de --missing       # Single locale with missing keys

# Auto-translate missing keys (dry-run)
foodshare-i18n translate de

# Auto-translate and apply
foodshare-i18n translate de --apply

# Sync all locales
foodshare-i18n sync                     # Dry-run
foodshare-i18n sync --apply             # Apply changes

# Benchmark endpoints
foodshare-i18n bench --count 10 --locale en

# List supported locales
foodshare-i18n locales
```

## Output Formats

```bash
# Text output (default)
foodshare-i18n status

# JSON output (for scripting)
foodshare-i18n --format json status
```

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
foodshare-i18n -v status

# Run tests
cargo test -p foodshare-i18n
```

## License

MIT
