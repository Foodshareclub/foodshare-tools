#!/usr/bin/env bash
# worktree-sweep.sh — daily cleanup of stray FoodShare worktrees.
#
# For each foodshare repo, inspects its IN-REPO worktrees (<repo>/.worktrees/).
# A worktree is removed when it is (a) clean (no uncommitted work), (b) pushed
# (re-checked immediately before removal), and (c) its card is done or archived.
# Dirty worktrees are saved to a wip/<name> branch + an artifacts/ backup before
# removal. Anything the script cannot place — unknown card, cross-board status
# collision, clean-but-unpushed, or out-of-repo — is KEPT (and counted/reported)
# rather than deleted.
#
# Board DBs are the cross-profile shared store under $HOME. They are NEVER
# derived from $HERMES_HOME: in agent shells $HERMES_HOME is the profile dir
# (e.g. ~/.hermes/profiles/architect), which has no board DBs, so the old
# lookup silently returned "unknown" for every card (a fail-silent no-op).
#
# Fail-loud: if zero board DBs are found, or none has a readable `tasks` table,
# the sweep exits 1 BEFORE touching any worktree. Per-card "unknown" (id found
# on no board) stays keep-safe and does not fail the sweep.
#
# DRY_RUN (default 1): report only, no mutations. DRY_RUN=0 executes.

set -euo pipefail

DRY_RUN="${DRY_RUN:-1}"
UMBRELLA="/Users/organic/dev/work/foodshare"
ARTIFACTS="$UMBRELLA/artifacts"
REPOS=(foodshare-web foodshare-backend foodshare-app foodshare-tools)
STAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
declare -a REMOVED=()
declare -a SAVED=()
declare -a KEPT=()
declare -a SKIPPED=()
# Unresolvable strays for the report card: "repo<TAB>path<TAB>reason" (t_8dec3a01).
declare -a STRAYS=()
REPORT="${REPORT:-1}"                 # 0 disables the report-to-card step
REPORT_BOARD="${REPORT_BOARD:-foodshare}"
REPORT_KEY="stray-worktrees"          # stable idempotency key of the single report card
HERMES_BIN="${HERMES_BIN:-$HOME/.hermes/hermes-agent/.hermes/bin/hermes}"
stray() { STRAYS+=("$1"$'\t'"$2"$'\t'"$3"); }

log() { printf '[worktree-sweep %s] %s\n' "$STAMP" "$*"; }
run() {
  if [ "$DRY_RUN" = "1" ]; then log "DRY: $*"; else log "EXEC: $*"; "$@"; fi
}

# --- Item 1: pin board DB locations to $HOME (never $HERMES_HOME) -------------
# Enumerate the canonical cross-profile shared store locations at run time.
BOARD_DBS=()
for db in "$HOME/.hermes/kanban.db" "$HOME"/.hermes/kanban/boards/*/kanban.db; do
  [ -f "$db" ] && BOARD_DBS+=("$db")
done

# --- Item 2: fail loud on zero board DBs / no readable tasks table ------------
if [ ${#BOARD_DBS[@]} -eq 0 ]; then
  log "ERROR: no board DBs found under $HOME/.hermes (checked kanban.db + kanban/boards/*/kanban.db) — refusing to sweep (fail-loud)."
  exit 1
fi
readable=0
for db in "${BOARD_DBS[@]}"; do
  if sqlite3 "$db" "SELECT 1 FROM tasks LIMIT 1;" >/dev/null 2>&1; then readable=1; break; fi
done
if [ "$readable" -eq 0 ]; then
  log "ERROR: no readable 'tasks' table in any board DB — refusing to sweep (fail-loud)."
  exit 1
fi
log "board DBs found (${#BOARD_DBS[@]}): ${BOARD_DBS[*]}"

# --- Item 1b: card status across all board DBs; cross-board collision -> keep --
# Look up a card id in every board DB. First hit wins. If the same id resolves
# to DIFFERENT statuses on more than one board (a cross-board collision), the
# card is treated as unresolved -> "collision" (keep + report, never remove).
CARD_STATE=""
CARD_DETAIL=""
board_label() {
  case "$1" in
    "$HOME/.hermes/kanban.db") printf 'default' ;;
    "$HOME/.hermes/kanban/boards/"*/kanban.db) printf '%s' "$(basename "$(dirname "$1")")" ;;
    *) printf '%s' "$(basename "$1")" ;;
  esac
}
card_state() {
  local id="$1" db st first="" seen=0 conflicts=0 detail="" label
  CARD_STATE=""; CARD_DETAIL=""
  [ -n "$id" ] || { CARD_STATE=unknown; return 0; }
  for db in "${BOARD_DBS[@]}"; do
    st="$(sqlite3 "$db" "SELECT status FROM tasks WHERE id='$id' LIMIT 1;" 2>/dev/null || true)"
    [ -n "$st" ] || continue
    label="$(board_label "$db")"
    detail+="${label}=${st} "
    if [ "$seen" -eq 0 ]; then first="$st"; seen=1
    elif [ "$st" != "$first" ]; then conflicts=1; fi
  done
  CARD_DETAIL="${detail% }"
  if [ "$conflicts" -eq 1 ]; then CARD_STATE=collision; else CARD_STATE="${first:-unknown}"; fi
}

card_id_from_path() {
  printf '%s\n' "$(basename "$1")" | grep -oE 't_[0-9a-f]{4,}' | head -1 || true
}
is_clean() { [ -z "$( { git -C "$1" status --porcelain 2>/dev/null || true; } )" ]; }
is_pushed() {
  local n
  n="$( { git -C "$1" branch -r --contains HEAD 2>/dev/null || true; } | wc -l | tr -d ' ' )"
  [ "$n" -gt 0 ]
}

for repo in "${REPOS[@]}"; do
  rdir="$UMBRELLA/$repo"
  if [ ! -e "$rdir/.git" ]; then log "skip $repo (no git)"; continue; fi
  wt_paths="$( { git -C "$rdir" worktree list --porcelain 2>/dev/null || true; } | grep '^worktree ' | sed 's/^worktree //' || true )"
  while IFS= read -r wt_path; do
    [ -n "$wt_path" ] || continue
    [ "$wt_path" = "$rdir" ] && continue   # main working tree
    # --- Item 3: scope to in-repo worktrees only; skip (count) everything else.
    # Harness workspaces (~/.hermes/kanban/boards/*/workspaces/...) and profile
    # scratch (~/.hermes/profiles/*/cache/scratch/...) are out of bounds: the
    # harness owns them and done-card harness workspaces still hold evidence the
    # completion-integrity gate re-verifies.
    case "$wt_path" in
      "$rdir/.worktrees/"*) : ;;
      *) SKIPPED+=("$wt_path"); stray "$repo" "$wt_path" "out-of-repo (outside <repo>/.worktrees; never touched)"; continue ;;
    esac
    # --- Acceptance G(a)/(b): explicit protected-prefix guard (defense in depth).
    # The in-repo scope above already confines removal to <repo>/.worktrees/, which
    # is structurally disjoint from $HOME/.hermes/** and the umbrella artifacts/.
    # This guard encodes that disjointness as an explicit negative check ON TOP of
    # the positive scope, so a future scope change could never reach a
    # completion-evidence location. Unreachable by design today.
    case "$wt_path" in
      "$HOME"/.hermes/*)
        log "$repo :: $(basename "$wt_path")  PROTECTED (under $HOME/.hermes/**) — kept, never removed (guard G(a))"
        KEPT+=("$wt_path (PROTECTED $HOME/.hermes/** — guard G(a), never removed)")
        continue ;;
      "$ARTIFACTS"/*)
        log "$repo :: $(basename "$wt_path")  PROTECTED (under umbrella artifacts/) — kept, never removed (guard G(b))"
        KEPT+=("$wt_path (PROTECTED umbrella artifacts/ — guard G(b), never removed)")
        continue ;;
    esac
    card_id="$(card_id_from_path "$wt_path")"
    card_state "$card_id"     # sets CARD_STATE + CARD_DETAIL (current shell, not a subshell)
    state="$CARD_STATE"
    clean=no; is_clean "$wt_path" && clean=yes
    log "$repo :: $(basename "$wt_path")  card=${card_id:-?} state=$state clean=$clean"
    case "$state" in
      done|archived)
        # --- Item 4: re-verify pushed immediately before the remove (atomicity).
        pushed=no; is_pushed "$wt_path" && pushed=yes
        if [ "$clean" = yes ] && [ "$pushed" = yes ]; then
          run git -C "$rdir" worktree remove "$wt_path"
          REMOVED+=("$wt_path")
        elif [ "$clean" = no ]; then
          name="$(basename "$wt_path")"
          wip="wip/sweep-$name"
          run git -C "$wt_path" add -A
          run git -C "$wt_path" -c user.name=worktree-sweep -c user.email=sweep@foodshare.local commit -q -m "sweep: WIP backup $name"
          run git -C "$wt_path" branch -f "$wip"
          run mkdir -p "$ARTIFACTS/worktree-backup/$name"
          run tar -C "$wt_path" -czf "$ARTIFACTS/worktree-backup/$name/worktree.tar.gz" --exclude=.git .
          run git -C "$rdir" worktree remove --force "$wt_path"
          SAVED+=("$wt_path -> $wip + artifacts/worktree-backup/$name")
        else
          KEPT+=("$wt_path (card $card_id $state, clean=$clean pushed=$pushed)")
          stray "$repo" "$wt_path" "clean but unpushed (card $card_id $state)"
        fi
        ;;
      collision)
        # Item 1b: the report line names the boards + conflicting statuses.
        log "$repo :: $(basename "$wt_path")  COLLISION — ${CARD_DETAIL} — keep + report, never remove"
        KEPT+=("$wt_path (card $card_id CROSS-BOARD COLLISION [${CARD_DETAIL}] — keep + report, never remove)")
        stray "$repo" "$wt_path" "cross-board collision ($CARD_DETAIL)"
        ;;
      *)
        KEPT+=("$wt_path (card $card_id state=$state)")
        if [ "$state" = unknown ]; then
          if [ -z "$card_id" ]; then stray "$repo" "$wt_path" "no card id (free-form name)"
          else stray "$repo" "$wt_path" "card $card_id found on no board"; fi
        fi
        ;;
    esac
  done <<< "$wt_paths"
done

log "SUMMARY removed=${#REMOVED[@]} saved=${#SAVED[@]} kept=${#KEPT[@]} skipped_out_of_repo=${#SKIPPED[@]}"
if [ "$DRY_RUN" = "1" ]; then log "(dry run — no mutations)"; fi
if [ ${#SAVED[@]} -gt 0 ]; then for s in "${SAVED[@]}"; do log "saved: $s"; done; fi
if [ ${#KEPT[@]} -gt 0 ]; then for k in "${KEPT[@]}"; do log "kept:  $k"; done; fi
if [ ${#SKIPPED[@]} -gt 0 ]; then log "skipped (out-of-repo, never touched): ${#SKIPPED[@]} worktrees"; fi

# --- Report-to-card (t_8dec3a01): surface the unresolvable strays on the board ----
# Spec: "report anything it isn't sure about to a board card instead of deleting it".
# Bars from the card:
#   * LAST step, after the deletion pass; every row is re-checked against a fresh
#     `git worktree list`, so the card can never list a since-deleted phantom.
#   * exactly ONE card, found by the stable idempotency key; the body is REPLACED each
#     run (latest inventory, never merged with an older one).
#   * kept non-dispatchable (blocked) with the "not actionable" marker.
#   * scoped: only this card is written, through the hermes kanban CLI.
#   * disjoint from deletion: it reads only STRAYS, runs in a subshell, and a failure
#     here is logged without changing the sweep's exit status or keep-safe behaviour.
report_strays() {
  set +e
  local board_db="$HOME/.hermes/kanban/boards/$REPORT_BOARD/kanban.db"
  [ -f "$board_db" ] || { log "report: board DB $board_db missing — skipped"; return 1; }
  [ -x "$HERMES_BIN" ] || { log "report: $HERMES_BIN not executable — skipped"; return 1; }

  # Post-deletion snapshot: every worktree path that exists now, across the repos.
  local current repo rdir
  current="$(for repo in "${REPOS[@]}"; do
    rdir="$UMBRELLA/$repo"
    [ -e "$rdir/.git" ] && { git -C "$rdir" worktree list --porcelain 2>/dev/null | sed -n 's/^worktree //p'; }
  done)"

  local rows="" n=0 nfree=0 nfound=0 ncoll=0 nunpushed=0 nout=0 legacy="" entry r p why
  if [ ${#STRAYS[@]} -gt 0 ]; then
    for entry in "${STRAYS[@]}"; do
      IFS=$'\t' read -r r p why <<< "$entry"
      grep -qxF -- "$p" <<< "$current" || continue   # gone after the deletion pass
      n=$((n + 1))
      rows+="| $n | $r | $(basename "$p") | \`$p\` | $why |"$'\n'
      case "$why" in
        "no card id"*) nfree=$((nfree + 1)); legacy+="- $r: \`$(basename "$p")\` → rename to \`<card-id>-<purpose>\` if a card maps, otherwise check for unpushed work and remove by hand"$'\n' ;;
        card*"found on no board") nfound=$((nfound + 1)) ;;
        "cross-board collision"*) ncoll=$((ncoll + 1)) ;;
        "clean but unpushed"*) nunpushed=$((nunpushed + 1)) ;;
        out-of-repo*) nout=$((nout + 1)) ;;
      esac
    done
  fi

  local body
  body="auto-upserted stray-worktrees report — not actionable, do not dispatch or unblock.

**Generated:** $STAMP by worktree-sweep.sh (post-deletion snapshot, DRY_RUN=$DRY_RUN)
**Board:** $REPORT_BOARD · **Removed this run:** ${#REMOVED[@]} · **Saved to wip/:** ${#SAVED[@]}
**Unresolvable strays: $n** — free-form/no card id $nfree · card not found $nfound · cross-board collision $ncoll · clean-but-unpushed $nunpushed · out-of-repo $nout

Each row was re-checked against \`git worktree list\` after the deletion pass. The sweep never deletes these; resolve them individually.

## Unresolvable strays
| # | repo | name | path | reason |
|---|------|------|------|--------|
${rows:-| - | - | - | - | none |
}
## Legacy free-form inventory (individual retirement)
${legacy:-none
}"

  local id
  id="$(sqlite3 -readonly "$board_db" "SELECT id FROM tasks WHERE idempotency_key='$REPORT_KEY' AND status<>'archived' ORDER BY created_at LIMIT 1;" 2>/dev/null)"
  if [ -z "$id" ]; then
    id="$(printf '%s' "$body" | "$HERMES_BIN" kanban --board "$REPORT_BOARD" create "Stray worktrees report" \
      --body-file - --idempotency-key "$REPORT_KEY" --json 2>/dev/null \
      | python3 -c 'import json,sys; print(json.load(sys.stdin).get("id",""))' 2>/dev/null)"
    [ -n "$id" ] || { log "report: could not create the report card"; return 1; }
    log "report: created $id"
  else
    "$HERMES_BIN" kanban --board "$REPORT_BOARD" edit "$id" --body "$body" >/dev/null 2>&1 \
      || { log "report: could not update $id"; return 1; }
    log "report: updated $id (body replaced)"
  fi

  local status
  status="$(sqlite3 -readonly "$board_db" "SELECT status FROM tasks WHERE id='$id';" 2>/dev/null)"
  case "$status" in
    blocked|scheduled) : ;;
    *) "$HERMES_BIN" kanban --board "$REPORT_BOARD" block "$id" \
         "auto-upserted stray-worktrees report — not actionable, do not dispatch or unblock" >/dev/null 2>&1 \
         && log "report: $id was $status -> blocked (non-dispatchable)" \
         || { log "report: could not block $id (status $status)"; return 1; } ;;
  esac
  log "report: $id lists $n unresolvable strays"
}

if [ "$REPORT" = "1" ]; then
  ( report_strays ) || log "report-to-card step failed — deletion results above are unaffected"
fi
