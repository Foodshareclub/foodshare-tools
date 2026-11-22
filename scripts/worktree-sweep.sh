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
      *) SKIPPED+=("$wt_path"); continue ;;
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
        fi
        ;;
      collision)
        # Item 1b: the report line names the boards + conflicting statuses.
        log "$repo :: $(basename "$wt_path")  COLLISION — ${CARD_DETAIL} — keep + report, never remove"
        KEPT+=("$wt_path (card $card_id CROSS-BOARD COLLISION [${CARD_DETAIL}] — keep + report, never remove)")
        ;;
      *)
        KEPT+=("$wt_path (card $card_id state=$state)")
        ;;
    esac
  done <<< "$wt_paths"
done

log "SUMMARY removed=${#REMOVED[@]} saved=${#SAVED[@]} kept=${#KEPT[@]} skipped_out_of_repo=${#SKIPPED[@]}"
if [ "$DRY_RUN" = "1" ]; then log "(dry run — no mutations)"; fi
if [ ${#SAVED[@]} -gt 0 ]; then for s in "${SAVED[@]}"; do log "saved: $s"; done; fi
if [ ${#KEPT[@]} -gt 0 ]; then for k in "${KEPT[@]}"; do log "kept:  $k"; done; fi
if [ ${#SKIPPED[@]} -gt 0 ]; then log "skipped (out-of-repo, never touched): ${#SKIPPED[@]} worktrees"; fi
