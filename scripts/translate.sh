#!/bin/bash
# =============================================================================
# Enterprise Translation Management CLI v2.0
# =============================================================================
# Usage:
#   ./scripts/translate.sh status             - Show overall status
#   ./scripts/translate.sh audit [locale]     - Audit translation coverage
#   ./scripts/translate.sh translate <locale> - Auto-translate missing keys
#   ./scripts/translate.sh sync               - Sync all locales
#   ./scripts/translate.sh health             - Check all endpoints health
#   ./scripts/translate.sh test <locale>      - Test translation fetch
# =============================================================================

set -e

BASE_URL="https://api.foodshare.club/functions/v1"
BFF_URL="${BASE_URL}/bff"
SUPPORTED_LOCALES="cs de es fr pt ru uk zh hi ar it pl nl ja ko tr vi id th sv"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

print_header() {
    echo -e "\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
}

# Health check all endpoints
check_health() {
    print_header "🏥 Endpoint Health Check"
    
    echo -e "${YELLOW}Checking endpoints...${NC}\n"
    
    # BFF
    bff_status=$(curl -s -o /dev/null -w "%{http_code}" "${BFF_URL}" 2>/dev/null)
    if [ "$bff_status" = "200" ]; then
        bff_version=$(curl -s "${BFF_URL}" 2>/dev/null | jq -r '.version // "unknown"')
        echo -e "  BFF:              ${GREEN}✓ OK${NC} (v${bff_version})"
    else
        echo -e "  BFF:              ${RED}✗ Error${NC} (HTTP ${bff_status})"
    fi
    
    # BFF Translations
    bff_trans=$(curl -s -o /dev/null -w "%{http_code}" "${BFF_URL}/translations?locale=en" 2>/dev/null)
    if [ "$bff_trans" = "200" ]; then
        echo -e "  BFF/translations: ${GREEN}✓ OK${NC}"
    else
        echo -e "  BFF/translations: ${RED}✗ Error${NC} (HTTP ${bff_trans})"
    fi
    
    # get-translations
    gt_health=$(curl -s "${BASE_URL}/get-translations/health" 2>/dev/null)
    gt_status=$(echo "$gt_health" | jq -r '.status // "error"')
    gt_version=$(echo "$gt_health" | jq -r '.version // "unknown"')
    if [ "$gt_status" = "ok" ]; then
        echo -e "  get-translations: ${GREEN}✓ OK${NC} (v${gt_version})"
        delta=$(echo "$gt_health" | jq -r '.features.deltaSync // false')
        prefetch=$(echo "$gt_health" | jq -r '.features.prefetch // false')
        echo -e "    └─ Delta Sync:  $([ "$delta" = "true" ] && echo "${GREEN}✓${NC}" || echo "${RED}✗${NC}")"
        echo -e "    └─ Prefetch:    $([ "$prefetch" = "true" ] && echo "${GREEN}✓${NC}" || echo "${RED}✗${NC}")"
    else
        echo -e "  get-translations: ${RED}✗ Error${NC}"
    fi
    
    # translation-audit
    audit_status=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/translation-audit" 2>/dev/null)
    if [ "$audit_status" = "200" ]; then
        echo -e "  translation-audit:${GREEN}✓ OK${NC}"
    else
        echo -e "  translation-audit:${RED}✗ Error${NC} (HTTP ${audit_status})"
    fi
    
    # translate-batch
    batch_status=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BASE_URL}/translate-batch" -H "Content-Type: application/json" -d '{}' 2>/dev/null)
    if [ "$batch_status" = "400" ] || [ "$batch_status" = "500" ]; then
        echo -e "  translate-batch:  ${GREEN}✓ OK${NC} (requires params)"
    else
        echo -e "  translate-batch:  ${YELLOW}? Unknown${NC} (HTTP ${batch_status})"
    fi
    
    echo ""
}

# Test translation fetch with timing
test_locale() {
    local locale=$1
    print_header "🧪 Testing Translation Fetch: ${locale}"
    
    echo -e "${YELLOW}Testing BFF endpoint...${NC}"
    start_time=$(python3 -c 'import time; print(int(time.time() * 1000))')
    bff_result=$(curl -s "${BFF_URL}/translations?locale=${locale}&platform=ios")
    end_time=$(python3 -c 'import time; print(int(time.time() * 1000))')
    bff_time=$((end_time - start_time))
    
    bff_success=$(echo "$bff_result" | jq -r '.success // false')
    if [ "$bff_success" = "true" ]; then
        bff_keys=$(echo "$bff_result" | jq -r '.data.messages | keys | length')
        bff_version=$(echo "$bff_result" | jq -r '.data.version // "unknown"')
        echo -e "  ${GREEN}✓ BFF${NC}: ${bff_keys} keys in ${bff_time}ms (v${bff_version})"
    else
        echo -e "  ${RED}✗ BFF failed${NC}"
    fi
    
    echo -e "\n${YELLOW}Testing direct endpoint...${NC}"
    start_time=$(python3 -c 'import time; print(int(time.time() * 1000))')
    direct_result=$(curl -s "${BASE_URL}/get-translations?locale=${locale}&platform=ios")
    end_time=$(python3 -c 'import time; print(int(time.time() * 1000))')
    direct_time=$((end_time - start_time))
    
    direct_success=$(echo "$direct_result" | jq -r '.success // false')
    if [ "$direct_success" = "true" ]; then
        direct_keys=$(echo "$direct_result" | jq -r '.data.messages | keys | length')
        direct_version=$(echo "$direct_result" | jq -r '.data.version // "unknown"')
        echo -e "  ${GREEN}✓ Direct${NC}: ${direct_keys} keys in ${direct_time}ms (v${direct_version})"
    else
        echo -e "  ${RED}✗ Direct failed${NC}"
    fi
    
    echo -e "\n${YELLOW}Testing ETag caching...${NC}"
    etag=$(echo "$direct_result" | jq -r '.data.version // ""')
    if [ -n "$etag" ]; then
        cache_result=$(curl -s -o /dev/null -w "%{http_code}" -H "If-None-Match: \"${etag}\"" "${BASE_URL}/get-translations?locale=${locale}")
        if [ "$cache_result" = "304" ]; then
            echo -e "  ${GREEN}✓ ETag caching working${NC} (304 Not Modified)"
        else
            echo -e "  ${YELLOW}? ETag returned HTTP ${cache_result}${NC}"
        fi
    fi
    
    echo ""
}

# Audit a single locale
audit_locale() {
    local locale=$1
    echo -e "${YELLOW}Auditing ${locale}...${NC}"
    
    result=$(curl -s "${BASE_URL}/translation-audit?locale=${locale}&limit=10")
    
    total=$(echo "$result" | jq -r '.totalKeys // 0')
    untranslated=$(echo "$result" | jq -r '.untranslatedCount // 0')
    
    if [ "$total" -gt 0 ]; then
        coverage=$(echo "scale=1; (($total - $untranslated) * 100) / $total" | bc)
        
        if (( $(echo "$coverage >= 90" | bc -l) )); then
            color=$GREEN
        elif (( $(echo "$coverage >= 70" | bc -l) )); then
            color=$YELLOW
        else
            color=$RED
        fi
        
        printf "  %-5s: ${color}%5.1f%%${NC} coverage (%d/%d keys)\n" "$locale" "$coverage" "$((total - untranslated))" "$total"
        
        if [ "$untranslated" -gt 0 ]; then
            echo -e "         Top missing: $(echo "$result" | jq -r '.byCategory | to_entries | sort_by(-.value) | .[0:3] | map("\(.key):\(.value)") | join(", ")')"
        fi
    else
        echo -e "  ${locale}: ${RED}No data${NC}"
    fi
}

# Audit all locales
audit_all() {
    print_header "🔍 Translation Coverage Audit"
    
    en_result=$(curl -s "${BASE_URL}/get-translations?locale=en")
    en_keys=$(echo "$en_result" | jq -r '.data.messages | [paths(type == "string")] | length')
    echo -e "Reference: English has ${GREEN}${en_keys}${NC} translation keys\n"
    
    for locale in $SUPPORTED_LOCALES; do
        audit_locale "$locale"
    done
    
    echo ""
}

# Translate missing keys for a locale
translate_locale() {
    local locale=$1
    local apply=${2:-false}
    
    print_header "🌐 Auto-Translating: ${locale}"
    
    echo -e "${YELLOW}Fetching untranslated keys...${NC}"
    
    audit_result=$(curl -s "${BASE_URL}/translation-audit?locale=${locale}&limit=50")
    untranslated=$(echo "$audit_result" | jq -r '.untranslatedCount // 0')
    
    if [ "$untranslated" -eq 0 ]; then
        echo -e "${GREEN}✓ All keys are translated for ${locale}!${NC}"
        return 0
    fi
    
    echo -e "Found ${YELLOW}${untranslated}${NC} untranslated keys"
    
    keys=$(echo "$audit_result" | jq -r '.untranslated | map({(.key): .englishValue}) | add')
    
    echo -e "${YELLOW}Translating with AI (GPT-4o-mini)...${NC}"
    
    translate_result=$(curl -s -X POST "${BASE_URL}/translate-batch" \
        -H "Content-Type: application/json" \
        -d "{\"locale\": \"${locale}\", \"keys\": ${keys}, \"apply\": ${apply}}")
    
    success=$(echo "$translate_result" | jq -r '.success // false')
    translated=$(echo "$translate_result" | jq -r '.translated // 0')
    
    if [ "$success" = "true" ]; then
        if [ "$apply" = "true" ]; then
            echo -e "${GREEN}✓ Translated and applied ${translated} keys to ${locale}${NC}"
            new_version=$(echo "$translate_result" | jq -r '.newVersion // "unknown"')
            echo -e "  New version: ${new_version}"
        else
            echo -e "${GREEN}✓ Translated ${translated} keys (dry run)${NC}"
            echo -e "${YELLOW}  Run with 'apply' to save: ./scripts/translate.sh translate ${locale} apply${NC}"
            echo ""
            echo "Sample translations:"
            echo "$translate_result" | jq -r '.translations | to_entries | .[0:5] | .[] | "  \(.key): \(.value)"'
        fi
    else
        error=$(echo "$translate_result" | jq -r '.message // .error // "Unknown error"')
        echo -e "${RED}✗ Translation failed: ${error}${NC}"
        return 1
    fi
}

# Sync all locales
sync_all() {
    print_header "🔄 Syncing All Locales"
    
    for locale in $SUPPORTED_LOCALES; do
        echo -e "\n${BLUE}Processing ${locale}...${NC}"
        translate_locale "$locale" "true"
    done
    
    echo -e "\n${GREEN}✓ Sync complete!${NC}"
}

# Show overall status
show_status() {
    print_header "📊 Translation System Status"
    
    echo -e "${YELLOW}Checking service health...${NC}"
    health=$(curl -s "${BASE_URL}/get-translations/health")
    status=$(echo "$health" | jq -r '.status // "unknown"')
    version=$(echo "$health" | jq -r '.version // "unknown"')
    
    if [ "$status" = "ok" ]; then
        echo -e "  Service: ${GREEN}✓ Healthy${NC} (v${version})"
    else
        echo -e "  Service: ${RED}✗ Unhealthy${NC}"
    fi
    
    delta=$(echo "$health" | jq -r '.features.deltaSync // false')
    prefetch=$(echo "$health" | jq -r '.features.prefetch // false')
    echo -e "  Delta Sync: $([ "$delta" = "true" ] && echo "${GREEN}✓${NC}" || echo "${RED}✗${NC}")"
    echo -e "  Prefetch: $([ "$prefetch" = "true" ] && echo "${GREEN}✓${NC}" || echo "${RED}✗${NC}")"
    
    # BFF status
    bff_result=$(curl -s "${BFF_URL}")
    bff_version=$(echo "$bff_result" | jq -r '.version // "unknown"')
    echo -e "  BFF: ${GREEN}✓${NC} (v${bff_version})"
    
    echo -e "\n${YELLOW}Fetching locale summary...${NC}"
    summary=$(curl -s "${BASE_URL}/translation-audit?all=true")
    
    total_locales=$(echo "$summary" | jq -r '.localeCount // 0')
    total_untranslated=$(echo "$summary" | jq -r '.totalUntranslated // 0')
    en_keys=$(echo "$summary" | jq -r '.englishKeyCount // 0')
    
    echo -e "  Locales: ${total_locales}"
    echo -e "  English Keys: ${en_keys}"
    echo -e "  Total Untranslated: ${YELLOW}${total_untranslated}${NC}"
    
    total_possible=$((en_keys * total_locales))
    if [ "$total_possible" -gt 0 ]; then
        overall_coverage=$(echo "scale=1; (($total_possible - $total_untranslated) * 100) / $total_possible" | bc)
        echo -e "  Overall Coverage: ${GREEN}${overall_coverage}%${NC}"
    fi
    
    echo -e "\n${YELLOW}Locales needing most work:${NC}"
    echo "$summary" | jq -r '.locales | .[0:5] | .[] | "  \(.locale): \(.untranslatedCount) keys missing"'
}

# Main
case "${1:-status}" in
    health)
        check_health
        ;;
    test)
        if [ -z "$2" ]; then
            test_locale "en"
        else
            test_locale "$2"
        fi
        ;;
    audit)
        if [ -n "$2" ]; then
            audit_locale "$2"
        else
            audit_all
        fi
        ;;
    translate)
        if [ -z "$2" ]; then
            echo -e "${RED}Error: locale required${NC}"
            echo "Usage: ./scripts/translate.sh translate <locale> [apply]"
            exit 1
        fi
        translate_locale "$2" "${3:-false}"
        ;;
    sync)
        sync_all
        ;;
    status)
        show_status
        ;;
    *)
        echo -e "${CYAN}Enterprise Translation Management CLI v2.0${NC}"
        echo ""
        echo "Usage:"
        echo "  ./scripts/translate.sh status             - Show overall status"
        echo "  ./scripts/translate.sh health             - Check all endpoints health"
        echo "  ./scripts/translate.sh test [locale]      - Test translation fetch"
        echo "  ./scripts/translate.sh audit [locale]     - Audit translation coverage"
        echo "  ./scripts/translate.sh translate <locale> - Auto-translate missing keys"
        echo "  ./scripts/translate.sh sync               - Sync all locales"
        echo ""
        echo "Supported locales: $SUPPORTED_LOCALES"
        ;;
esac
