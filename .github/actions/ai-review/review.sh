#!/usr/bin/env bash
# AI-powered review / test-gap analysis. Groq primary, Z.AI fallback.
# Zero deps beyond curl + jq (preinstalled on GitHub runners).
set -uo pipefail

MODE="${INPUT_MODE:-review}"
SINCE="${INPUT_SINCE:-24 hours}"
MAX_DIFF_BYTES="${INPUT_MAX_DIFF_BYTES:-60000}"
GROQ_MODEL="${GROQ_MODEL:-openai/gpt-oss-120b}"
ZAI_MODEL="${ZAI_MODEL:-glm-4.6}"

summary_file="${GITHUB_STEP_SUMMARY:-/dev/stdout}"

collect_context() {
  local base
  base="$(git merge-base HEAD origin/main 2>/dev/null || git rev-parse origin/main 2>/dev/null || echo "")"
  local diff
  if [ -n "$base" ] && [ "${PR_NUMBER:-}" != "" ]; then
    diff="$(git diff --unified=1 "$base"...HEAD 2>/dev/null)"
    echo "### PR diff vs main" >&2
  else
    diff="$(git log --since="$SINCE" -p --stat --no-color 2>/dev/null)"
    [ -z "$diff" ] && diff="(no commits in the last $SINCE — reporting on current tree state)"
    echo "### Scheduled window: last $SINCE" >&2
  fi
  if [ "${#diff}" -gt "$MAX_DIFF_BYTES" ]; then
    diff="${diff:0:$MAX_DIFF_BYTES}
... [truncated ${#diff} -> ${MAX_DIFF_BYTES} bytes]"
  fi
  printf '%s\n' "$diff"
}

build_prompt() {
  local context="$1"
  if [ "$MODE" = "test-gap" ]; then
    cat <<PROMPT
You are a senior QA architect auditing a Foodshare repository (Next.js web, Deno Supabase backend, Skip Fuse mobile, Rust tooling). E2E stack: Playwright (web), Maestro (mobile), deno test (backend), cargo test (tooling).

Below is the recent change context. Produce a concise markdown report:
1. **Coverage gaps** — changed behaviour lacking unit/e2e coverage; name the exact file and the test you would add (framework + file path + test title).
2. **Flaky-risk notes** — timing/network-dependent changes needing retries or fakes.
3. **Top 5 prioritized actions** as a checklist.
Be specific. No praise, no filler.

CHANGE CONTEXT:
\`\`\`
$context
\`\`\`
PROMPT
  else
    cat <<PROMPT
You are a meticulous staff engineer reviewing changes in a Foodshare repository (Next.js web, Deno Supabase backend, Skip Fuse mobile, Rust tooling). House rules: RLS everywhere, structured logging (never console.log), CI/CD-first deploys, secrets only via Vault/GitHub Secrets.

Review the change context below. Output markdown:
- A table of findings: | Severity (critical/major/minor/nit) | File | Issue | Suggested fix |
- One short paragraph: overall risk to production.
Max 12 findings; only real issues, no style nits unless they break house rules.

CHANGE CONTEXT:
\`\`\`
$context
\`\`\`
PROMPT
  fi
}

call_provider() {
  local url="$1" model="$2" key="$3" prompt="$4" payload resp body content
  local extra='{}'
  # GLM reasoning models: disable thinking so tokens go to real content
  case "$url" in *z.ai*) extra='{"thinking":{"type":"disabled"}}' ;; esac
  payload="$(jq -nc --arg m "$model" --arg p "$prompt" --argjson x "$extra" \
    '{model:$m,messages:[{role:"user",content:$p}],temperature:0.2,max_tokens:4096} + $x')"
  resp="$(curl -sS --max-time 180 -w '\n%{http_code}' -X POST "$url" \
    -H "Authorization: Bearer $key" -H "Content-Type: application/json" -d "$payload")" || return 1
  local code="${resp##*$'\n'}"
  body="${resp%$'\n'*}"
  [ "$code" = "200" ] || { echo "provider $model HTTP $code: ${body:0:200}" >&2; return 1; }
  # content first; fall back to reasoning_content (reasoning models may put
  # everything there when token budget is exhausted)
  content="$(printf '%s' "$body" | jq -r '.choices[0].message.content // empty')" || return 1
  if [ -z "$content" ]; then
    content="$(printf '%s' "$body" | jq -r '.choices[0].message.reasoning_content // empty')" || return 1
  fi
  [ -n "$content" ] && printf '%s' "$content"
}

CONTEXT="$(collect_context)"
PROMPT="$(build_prompt "$CONTEXT")"

echo "## 🤖 AI $MODE ($(date -u +%Y-%m-%d))" > "$summary_file"

OUTPUT=""
if [ -n "${GROQ_API_KEY:-}" ]; then
  echo "Trying Groq ($GROQ_MODEL)..." >&2
  OUTPUT="$(call_provider https://api.groq.com/openai/v1/chat/completions "$GROQ_MODEL" "$GROQ_API_KEY" "$PROMPT")" && \
    echo "_Provider: Groq / ${GROQ_MODEL}" >> "$summary_file" || OUTPUT=""
fi
if [ -z "$OUTPUT" ] && [ -n "${ZAI_API_KEY:-}" ]; then
  echo "Falling back to Z.AI ($ZAI_MODEL)..." >&2
  OUTPUT="$(call_provider https://api.z.ai/api/paas/v4/chat/completions "$ZAI_MODEL" "$ZAI_API_KEY" "$PROMPT")" && \
    echo "_Provider: Z.AI / ${ZAI_MODEL}" >> "$summary_file" || OUTPUT=""
fi

if [ -z "$OUTPUT" ]; then
  echo "All AI providers failed." >> "$summary_file"
  echo "::warning::AI review unavailable (both providers failed or keys missing)"
  [ "${FAIL_ON_ERROR:-false}" = "true" ] && exit 1
  exit 0
fi

printf '%s\n' "$OUTPUT" >> "$summary_file"

if [ -n "${PR_NUMBER:-}" ] && command -v gh >/dev/null 2>&1; then
  printf '%s\n' "$OUTPUT" > /tmp/ai-review.md
  gh pr comment "$PR_NUMBER" --repo "$REPO" --body-file /tmp/ai-review.md >/dev/null 2>&1 || \
    echo "::warning::Failed to post PR comment" >&2
fi
