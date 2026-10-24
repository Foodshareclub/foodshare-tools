#!/usr/bin/env bash
# Cross-repo compatibility checker.
# Verifies: vendored supabase drift, production health endpoints, latest CI
# conclusions across the Foodshareclub repos. Writes a markdown matrix to
# $GITHUB_STEP_SUMMARY when available. Exits non-zero on failed or unverified checks unless
# SOFT_FAIL=1.
#
# Env / args:
#   WEB_DIR      path to foodshare-web checkout   (enables drift check)
#   BACKEND_DIR  path to foodshare-backend checkout
#   API_HEALTH   default https://api.foodshare.club/functions/v1/api-v1-health
#   STUDIO_URL   default https://studio.foodshare.club
#   CHECK_REPOS  space-separated repos for CI status (default: all four)
#   IGNORE_CI_FAIL  space-separated repos whose CI failure is a known
#                   external issue -> demoted to WARN instead of FAIL
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
  # These are the paths explicitly vendored by foodshare-web/.gitignore.
  # Backend-only oracle data, SQL test harnesses and runtime files are not
  # part of the web snapshot and must not create permanent false drift.
  drift=0
  for path in config.toml seed.sql migrations functions; do
    if [ ! -e "$WEB_DIR/supabase/$path" ] || [ ! -e "$BACKEND_DIR/supabase/$path" ]; then
      row "Vendored copy drift" "FAIL" "Missing required vendored path: $path"
      drift=1
    elif ! diff -rq --exclude=node_modules --exclude=.DS_Store \
      "$WEB_DIR/supabase/$path" "$BACKEND_DIR/supabase/$path" >/dev/null 2>&1; then
      row "Vendored copy drift" "FAIL" "web supabase/$path diverged from backend; re-sync required"
      drift=1
    fi
  done
  if [ "$drift" = 0 ]; then
    row "Vendored copy drift" "PASS" "All declared web Supabase paths match backend"
  else
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

# 3. Required pipeline at each repository's current main commit ------------
# The Bun helper selects the canonical workflow and exact commit. An unrelated
# successful API probe cannot turn a failed deployment pipeline green.
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export CHECK_REPOS
export IGNORE_CI_FAIL="${IGNORE_CI_FAIL:-}"
if ci_rows="$(bun "$script_dir/compat-ci.ts")"; then
  ci_status=0
else
  ci_status=$?
fi
if [ -n "$ci_rows" ]; then
  while IFS= read -r line; do ROWS+=("$line"); done <<< "$ci_rows"
else
  row "CI inspection" "FAIL" "No pipeline inspection results; Bun is required"
  FAIL=1
fi
if [ "$ci_status" -ne 0 ]; then FAIL=1; fi

# Report -------------------------------------------------------------------
report() {
  echo "## 🔗 Daily Compatibility Matrix ($(date -u +%Y-%m-%dT%H:%MZ))"$'\n'
  echo "| Check | Status | Detail |"
  echo "| ----- | ------ | ------ |"
  printf '%s\n' "${ROWS[@]}"
}

report | tee "${summary_file:-/dev/null}"

if [ "$FAIL" -eq 1 ] && [ "${SOFT_FAIL:-0}" != "1" ]; then
  echo "::error::Compatibility failed or is not yet verified — see matrix above"
  exit 1
fi
