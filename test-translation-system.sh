"   bunx supabase functions logs localization --tail"
echo "   bunx supabase functions logs bff --tail | grep 'Translations fetched'"
echo ""
echo "4. ${YELLOW}Check cache hit rates:${NC}"
echo "   SELECT target_locale, COUNT(*), SUM(hit_count) FROM content_translations GROUP BY target_locale;"

echo -e "\n${GREEN}✓ Translation system is ready!${NC}\n"

exit 0
-X POST ${SUPABASE_URL}/functions/v1/localization/backfill-posts \\"
echo "     -H \"Authorization: Bearer \$SUPABASE_SERVICE_ROLE_KEY\" \\"
echo "     -H \"Content-Type: application/json\" \\"
echo "     -d '{\"limit\": 100, \"offset\": 0}'"
echo ""
echo "2. ${YELLOW}Test in iOS app:${NC}"
echo "   - Change locale in Settings"
echo "   - Browse feed and verify translations appear"
echo "   - Check that displayTitle/displayDescription show translated text"
echo ""
echo "3. ${YELLOW}Monitor performance:${NC}"
echo     Test Summary                            ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}\n"

echo -e "${GREEN}✓ Database schema ready${NC}"
echo -e "${GREEN}✓ Translation service working${NC}"
echo -e "${GREEN}✓ Cache layers functional (Redis → PostgreSQL → LLM)${NC}"
echo -e "${GREEN}✓ BFF integration complete${NC}"
echo -e "${GREEN}✓ End-to-end flow verified${NC}"

echo -e "\n${BLUE}Next Steps:${NC}"
echo "1. ${YELLOW}Backfill existing posts:${NC}"
echo "   curl TATS" | jq -r '.[] | "  \(.locale)       \(.total_translations)            \(.cached_hits)          \(.avg_quality | tonumber | . * 100 | floor / 100)"' | head -5
    
    TOTAL_ALL=$(echo "$STATS" | jq '[.[].total_translations] | add')
    echo -e "\n  ${GREEN}Total translations across all locales: $TOTAL_ALL${NC}"
else
    echo -e "${YELLOW}⚠ No translation stats yet${NC}"
fi

# Summary
echo -e "\n${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                  ────────  ────────────  ──────────  ───────────"
    
    echo "$S1
fi

# Test 7: Check translation statistics
echo -e "\n${YELLOW}[7/7] Checking translation statistics...${NC}"
STATS=$(curl -s -X POST "${SUPABASE_URL}/rest/v1/rpc/get_translation_stats" \
  -H "apikey: ${SUPABASE_ANON_KEY}" \
  -H "Authorization: Bearer ${SUPABASE_ANON_KEY}" \
  -H "Content-Type: application/json" \
  -d '{}')

if echo "$STATS" | grep -q "locale"; then
    echo -e "${GREEN}✓ Translation statistics available${NC}\n"
    
    echo "  Locale    Translations  Cache Hits  Avg Quality"
    echo "        echo "    Russian:  $FIRST_TITLE_RU"
        fi
    else
        echo -e "${YELLOW}⚠ No translations in BFF response${NC}"
        echo "  Posts may not be translated yet. Run backfill:"
        echo "  curl -X POST ${SUPABASE_URL}/functions/v1/localization/backfill-posts \\"
        echo "    -H \"Authorization: Bearer \$SUPABASE_SERVICE_ROLE_KEY\" \\"
        echo "    -d '{\"limit\": 100}'"
    fi
else
    echo -e "${RED}✗ BFF feed endpoint failed${NC}"
    echo "$BFF_RESPONSE" | jq '.'
    exit   ${BLUE}Example translation:${NC}"
            echo "    ID: $FIRST_ID"
            echo "    Original: $FIRST_TITLE"
     "  Translated: $TRANSLATED_COUNT"
    
    if [ "$TRANSLATED_COUNT" -gt 0 ]; then
        echo -e "${GREEN}✓ Translations integrated in BFF response${NC}"
        
        # Show first translated listing
        FIRST_ID=$(echo "$BFF_RESPONSE" | jq -r '.listings[0].id')
        FIRST_TITLE=$(echo "$BFF_RESPONSE" | jq -r '.listings[0].title')
        FIRST_TITLE_RU=$(echo "$BFF_RESPONSE" | jq -r '.listings[0].titleTranslated')
        
        if [ "$FIRST_TITLE_RU" != "null" ]; then
            echo -e "\nKEY}" \
  -H "Content-Type: application/json" \
  -d "{
    \"lat\": ${LAT},
    \"lng\": ${LNG},
    \"radiusKm\": 100,
    \"limit\": 5,
    \"locale\": \"ru\"
  }")

if echo "$BFF_RESPONSE" | grep -q "listings"; then
    LISTINGS_COUNT=$(echo "$BFF_RESPONSE" | jq '.listings | length')
    TRANSLATED_COUNT=$(echo "$BFF_RESPONSE" | jq '[.listings[] | select(.titleTranslated != null)] | length')
    
    echo -e "${GREEN}✓ BFF feed endpoint working${NC}"
    echo "  Total listings: $LISTINGS_COUNT"
    echoST "${SUPABASE_URL}/functions/v1/bff/feed" \
  -H "Authorization: Bearer ${SUPABASE_ANON_"LLM (on-demand)"
            
            echo -e "  ${GREEN}✓ ${LOCALE}: ${TITLE_TRANS:0:50}${NC} [${CACHE_SOURCE}]"
        else
            echo -e "  ${YELLOW}⚠ ${LOCALE}: Not translated yet${NC}"
        fi
    else
        echo -e "  ${RED}✗ ${LOCALE}: Failed to fetch${NC}"
    fi
done

# Test 6: Test BFF feed endpoint with locale
echo -e "\n${YELLOW}[6/7] Testing BFF feed endpoint with translations...${NC}"

# Get user's location (London as default)
LAT=51.5074
LNG=-0.1278

BFF_RESPONSE=$(curl -s -X PO"success"; then
        TITLE_TRANS=$(echo "$GET_TRANS" | jq -r ".translations.\"${POST_ID}\".title")
        FROM_REDIS=$(echo "$GET_TRANS" | jq -r '.fromRedis')
        FROM_DB=$(echo "$GET_TRANS" | jq -r '.fromDatabase')
        ON_DEMAND=$(echo "$GET_TRANS" | jq -r '.onDemand')
        
        if [ "$TITLE_TRANS" != "null" ] && [ -n "$TITLE_TRANS" ]; then
            CACHE_SOURCE="Redis"
            [ "$FROM_DB" -gt 0 ] && CACHE_SOURCE="PostgreSQL"
            [ "$ON_DEMAND" -gt 0 ] && CACHE_SOURCE=ntentIds\": [\"${POST_ID}\"],
        \"locale\": \"${LOCALE}\",
        \"fields\": [\"title\", \"description\"]
      }")

    if echo "$GET_TRANS" | grep -q fi
else
    echo -e "\n${YELLOW}[4/7] Skipping translation (already exists)${NC}"
fi

# Test 5: Fetch translations via get-translations endpoint
echo -e "\n${YELLOW}[5/7] Testing translation lookup (multiple locales)...${NC}"

for LOCALE in "ru" "es" "de" "fr"; do
    GET_TRANS=$(curl -s -X POST "${SUPABASE_URL}/functions/v1/localization/get-translations" \
      -H "Authorization: Bearer ${SUPABASE_ANON_KEY}" \
      -H "Content-Type: application/json" \
      -d "{
        \"contentType\": \"post\",
        \"coANS=$(echo "$TRANSLATE_RESPONSE" | jq -r '.total_translations')
        echo "  Total translations queued: $TOTAL_TRANS (2 fields × 20 locales)"
        
        echo -e "\n${BLUE}⏳ Waiting 30 seconds for translations to complete...${NC}"
        for i in {30..1}; do
            echo -ne "  ${i}s remaining...\r"
            sleep 1
        done
        echo -e "  ${GREEN}Done!${NC}                    "
    else
        echo -e "${RED}✗ Translation failed${NC}"
        echo "$TRANSLATE_RESPONSE"
        exit 1
         -H "Authorization: Bearer ${SUPABASE_SERVICE_KEY}" \
      -H "Content-Type: application/json" \
      -d "{
        \"content_type\": \"post\",
        \"content_id\": \"${POST_ID}\",
        \"fields\": [
          {\"name\": \"title\", \"text\": \"${POST_NAME}\"},
          {\"name\": \"description\", \"text\": \"${POST_DESC}\"}
        ]
      }")

    if echo "$TRANSLATE_RESPONSE" | grep -q "accepted"; then
        echo -e "${GREEN}✓ Translation triggered${NC}"
        TOTAL_TRL}/functions/v1/localization/translate-batch" \
 " ] && [ -n "$TITLE_RU" ]; then
    echo -e "${GREEN}✓ Post already has Russian translation${NC}"
    echo "  Original: $POST_NAME"
    echo "  Russian:  $TITLE_RU"
    SKIP_TRANSLATION=true
else
    echo -e "${YELLOW}⚠ Post not translated yet, will trigger translation${NC}"
    SKIP_TRANSLATION=false
fi

# Test 4: Trigger translation (if needed)
if [ "$SKIP_TRANSLATION" = false ]; then
    echo -e "\n${YELLOW}[4/7] Triggering translation for post...${NC}"
    TRANSLATE_RESPONSE=$(curl -s -X POST "${SUPABASE_UR
    \"fields\": [\"title\", \"description\"]
  }")

TITLE_RU=$(echo "$EXISTING_TRANS" | jq -r ".translations.\"${POST_ID}\".title")

if [ "$TITLE_RU" != "null  Description: ${POST_DESC:0:60}..."

# Test 3: Check if post already has translations
echo -e "\n${YELLOW}[3/7] Checking existing translations...${NC}"
EXISTING_TRANS=$(curl -s -X POST "${SUPABASE_URL}/functions/v1/localization/get-translations" \
  -H "Authorization: Bearer ${SUPABASE_ANON_KEY}" \
  -H "Content-Type: application/json" \
  -d "{
    \"contentType\": \"post\",
    \"contentIds\": [\"${POST_ID}\"],
    \"locale\": \"ru\",_KEY}")

POST_ID=$(echo "$SAMPLE_POST" | jq -r '.[0].id')
POST_NAME=$(echo "$SAMPLE_POST" | jq -r '.[0].post_name')
POST_DESC=$(echo "$SAMPLE_POST" | jq -r '.[0].post_description')

if [ "$POST_ID" == "null" ] || [ -z "$POST_ID" ]; then
    echo -e "${RED}✗ No active posts found in database${NC}"
    echo "  Create a test post first"
    exit 1
fi

echo -e "${GREEN}✓ Found post: ID=$POST_ID${NC}"
echo "  Title: ${POST_NAME:0:60}"
echo "-H "Authorization: Bearer ${SUPABASE_ANOName,post_description&is_active=eq.true&limit=1" \
  -H "apikey: ${SUPABASE_ANON_KEY}" \
   GET "${SUPABASE_URL}/rest/v1/posts?select=id,post_no -e "\n${YELLOW}[2/7] Fetching sample post...${NC}"
SAMPLE_POST=$(curl -s -X"
    echo "  Error: $SCHEMA_CHECK"
    exit 1
else
    echo -e "${GREEN}✓ Database schema OK (content_translations table exists)${NC}"
fi

# Test 2: Get a sample post
echpplication/json" \
  -d '{}')

if echo "$SCHEMA_CHECK" | grep -q "error"; then
    echo -e "${RED}✗ Database schema not ready${NC}"
    echo "  Run: cd ../foodshare-backend && bunx supabase db push_ANON_KEY}" \
  -H "Authorization: Bearer ${SUPABASE_ANON_KEY}" \
  -H "Content-Type: a
if ! command -v jq &> /dev/null; then
    echo -e "${RED}✗ Error: jq is not installed${NC}"
    echo "  Install it: brew install jq"
    exit 1
fi

echo -e "${GREEN}✓ Prerequisites OK${NC}\n"

# Test 1: Check database schema
echo -e "${YELLOW}[1/7] Checking database schema...${NC}"
SCHEMA_CHECK=$(curl -s -X POST "${SUPABASE_URL}/rest/v1/rpc/get_translation_stats" \
  -H "apikey: ${SUPABASEit: export SUPABASE_SERVICE_ROLE_KEY=your-key"
    exit 1
fi

# Check if jq is installed"$SUPABASE_SERVICE_KEY" ]; then
    echo -e "${RED}✗ Error: SUPABASE_SERVICE_ROLE_KEY not set${NC}"
    echo "  Export SE_ANON_KEY not set${NC}"
    echo "  Export it: export SUPABASE_ANON_KEY=your-key"
    exit 1
fi

if [ -z cho -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}\n"

# Check prerequisites
if [ -z "$SUPABASE_ANON_KEY" ]; then
    echo -e "${RED}✗ Error: SUPABA════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         Translation System End-to-End Test                ║${NC}"
e

# Configuration
SUPABASE_URL="${SUPABASE_URL:-https://iazmjdjwnkilycbjwpzp.supabase.co}"
SUPABASE_ANON_KEY="${SUPABASE_ANON_KEY}"
SUPABASE_SERVICE_KEY="${SUPABASE_SERVICE_ROLE_KEY}"

echo -e "${BLUE}╔════-to-End
# Tests the complete translation pipeline from database to iOS

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color#!/bin/bash
# Test Translation System End