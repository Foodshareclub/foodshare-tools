#!/usr/bin/env bash
# Cross-repo compatibility checker.
# Verifies: vendored supabase drift, production health endpoints, latest CI
# conclusions across the Foodshareclub repos. Writes a markdown matrix to
# $GITHUB_STEP_SUMMARY when available. Exits non-zero on any FAIL unless
# SOFT_FAIL=1.
#
# Env / args:
#   WEB_DIR      path to foodshare-web checkout   (enables drift check)
#   BACKEND_DIR  path to foodshare-backend checkout
#   API_HEALTH   default https://api.foodshare.club/functions/v1/api-v1-health
#   STUDIO_URL   default https://studio.foodshare.club
#   CHECK_REPOS  space-separated repos for CI status (default: all four)
set -uo pipefail

WEB_DIR="${WEB_DIR:-}"
BACKEND_DIR="${BACKEND_DIR:-}"
API_HEALTH="${API_HEALTH:-https://api.foodshare.club/functions/v1/api-v1-health}"
STUDIO_URL="${STUDIO_URL:-https://studio.foodshare.club}"
CHECK_REPOS="${CHECK_REPOS:-Foodshareclub/foodshare-backend Foodshareclub/foodshare-web Foodshareclub/foodshare-app Foodshareclub/foodshare-tools}"
summary_file="${GITHUB_STEP_SUMMARY:-}"

declare -a ROWS=()
FAIL=0

row() { ROWS+=("| $1 | $2 | $3 |"); }

# 1. Vendored supabase drift -----------------------------------------------
if [ -n "$WEB_DIR" ] && [ -n "$BACKEND_DIR" ]; then
  if diff_out="$(diff -rq --exclude=node_modules --exclude=.DS_Store \
      "$WEB_DIR/supabase" "$BACKEND_DIR/supabase" 2>/dev/null)"; then
    row "Vendored copy drift" "PASS" "web \`supabase/\` matches backend"
  else
    count="$(printf '%s\n' "$diff_out" | grep -c 'differ\|Only in' || true)"
    row "Vendored copy drift" "FAIL" "\`web/supabase\` diverged from backend ($count paths). Re-sync required."
    FAIL=1
  fi
else
  row "Vendored copy drift" "SKIP" "WEB_DIR/BACKEND_DIR not provided"
fi

# 2. Production health endpoints -------------------------------------------
http_code="$(curl -sS -o /dev/null -w '%{http_code}' --max-time 20 "$API_HEALTH" 2>/dev/null)"
[ -z "$http_code" ] && http_code="transport-error"
if [ "$http_code" = "200" ]; then
  row "Backend API health" "PASS" "\`$API_HEALTH\` -> 200"
else
  row "Backend API health" "FAIL" "\`$API_HEALTH\` -> $http_code"
  FAIL=1
fi

studio_code="$(curl -sS -o /dev/null -w '%{http_code}' --max-time 20 "$STUDIO_URL" 2>/dev/null)"
[ -z "$studio_code" ] && studio_code="transport-error"
if [ "$studio_code" = "200" ] || [ "$studio_code" = "401" ] || [ "$studio_code" = "302" ]; then
  row "Studio reachable" "PASS" "\`$STUDIO_URL\` -> $studio_code"
else
  row "Studio reachable" "FAIL" "\`$STUDIO_URL\` -> $studio_code"
  FAIL=1
fi

# 3. Latest CI conclusion on main per repo ---------------------------------
for repo in $CHECK_REPOS; do
  if conclusion="$(gh api "repos/$repo/actions/runs?branch=main&per_page=1" \
      --jq '.workflow_runs[0].conclusion // "none"' 2>/dev/null)"; then
    if [ "$conclusion" = "success" ] || [ "$conclusion" = "skipped" ]; then
      row "CI: \`$repo\`" "PASS" "latest main run: $conclusion"
    elif [ "$conclusion" = "none" ]; then
      row "CI: \`$repo\`" "WARN" "no runs found on main"
    else
      row "CI: \`$repo\`" "FAIL" "latest main run: ${conclusion:-unknown}"
      [ "$conclusion" != "none" ] && FAIL=1
    fi
  else
    row "CI: \`$repo\`" "WARN" "gh api unavailable or repo unreachable"
  fi
done

# Report -------------------------------------------------------------------
report() {
  echo "## 🔗 Daily Compatibility Matrix ($(date -u +%Y-%m-%dT%H:%MZ))"$'\n'
  echo "| Check | Status | Detail |"
  echo "| ----- | ------ | ------ |"
  printf '%s\n' "${ROWS[@]}"
}

if [ -n "$summary_file" ]; then
  report >> "$summary_file"
else
  report
fi

if [ "$FAIL" -eq 1 ] && [ "${SOFT_FAIL:-0}" != "1" ]; then
  echo "::error::Compatibility check failed — see matrix above"
  exit 1
fi
