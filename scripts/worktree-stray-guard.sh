#!/bin/sh
# Worktree stray-guard (layer 2 of the worktree hygiene, card t_f1c4e587).
#
# A FoodShare worktree belongs at <repo>/.worktrees/<name>. Worktrees made
# anywhere else (the umbrella folder, /tmp, a sibling directory) are the
# strays that worktree-sweep.sh (layer 3) can only report, never clean.
#
# Installed as both hooks in each repo's shared .git/hooks, so every
# worktree of the repo runs it:
#   post-checkout  warns loudly (post-checkout cannot block a checkout)
#   pre-push       refuses to push from a stray worktree
# The main working tree and anything under <repo>/.worktrees/ pass.
# FOODSHARE_ALLOW_STRAY_PUSH=1 lets one push through deliberately.
#
# Install: scripts/install-worktree-stray-guard.sh

hook=$(basename "$0")

top=$(git rev-parse --show-toplevel 2>/dev/null) || exit 0
common=$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null) || exit 0
case "$common" in
  */.git) main=${common%/.git} ;;
  *) exit 0 ;; # bare repository or a layout this guard does not know
esac

real() { (cd "$1" 2>/dev/null && pwd -P); }
top=$(real "$top") || exit 0
main=$(real "$main") || exit 0

case "$top" in
  "$main" | "$main"/.worktrees/*) exit 0 ;;
esac

say() { printf '%s\n' "$@" >&2; }
say "" \
  "!! worktree-stray-guard: this worktree is outside $main/.worktrees/" \
  "!!   here:     $top" \
  "!!   expected: $main/.worktrees/<card-id>[-<purpose>]" \
  "!! Move it:  git worktree move \"$top\" \"$main/.worktrees/$(basename "$top")\""

if [ "$hook" = "pre-push" ]; then
  if [ "${FOODSHARE_ALLOW_STRAY_PUSH:-}" = "1" ]; then
    say "!! FOODSHARE_ALLOW_STRAY_PUSH=1: pushing anyway." ""
    exit 0
  fi
  say "!! Push refused. Set FOODSHARE_ALLOW_STRAY_PUSH=1 to push once from here." ""
  exit 1
fi
say ""
exit 0
