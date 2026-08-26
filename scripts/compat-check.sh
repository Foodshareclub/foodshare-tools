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
# Prefer gh when available; fall back to curl + GH_TOKEN (self-hosted
# runners may not ship the gh CLI).
gh_api() {
  local path="$1" jq_expr="$2"
  if command -v gh >/dev/null 2>&1; then
    gh api "$path" --jq "$jq_expr" 2>/dev/null
  elif [ -n "${GH_TOKEN:-}" ]; then
    curl -sS --max-time 20 -H "Authorization: Bearer $GH_TOKEN" \
      -H "Accept: application/vnd.github+json" \
      "https://api.github.com$path" |
      jq -r "$jq_expr" 2>/dev/null
  fi
}

for repo in $CHECK_REPOS; do
  # event=push: judge repos by their deploy pipelines, not by sibling
  # nightly runs (otherwise one red nightly marks every other repo red).
  if conclusion="$(gh_api "repos/$repo/actions/runs?branch=main&event=push&per_page=1" '.workflow_runs[0].conclusion // "none"')"; then
    if [ "$conclusion" = "success" ] || [ "$conclusion" = "skipped" ]; then
      row "CI: \`$repo\`" "PASS" "latest main run: $conclusion"
    elif [ "$conclusion" = "none" ]; then
      row "CI: \`$repo\`" "WARN" "no runs found on main"
    elif echo "$IGNORE_CI_FAIL" | tr ' ' '\n' | grep -qx "$repo"; then
      row "CI: \`$repo\`" "WARN" "known external failure (ignored): ${conclusion:-unknown}"
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

report | tee "${summary_file:-/dev/null}"

if [ "$FAIL" -eq 1 ] && [ "${SOFT_FAIL:-0}" != "1" ]; then
  echo "::error::Compatibility check failed — see matrix above"
  exit 1
fi
