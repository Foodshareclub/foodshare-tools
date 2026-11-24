#!/bin/sh
# Install worktree-stray-guard.sh as the post-checkout and pre-push hooks of
# each FoodShare repo's shared .git/hooks (every worktree of a repo uses
# them), and into the global git template so new clones get it too.
#
# Usage: scripts/install-worktree-stray-guard.sh [repo-dir ...]
# Default repos: the foodshare-* siblings of this repository.
#
# A hook of the same name that is not this guard (for example one written
# by `lefthook install`) is left alone and reported, never overwritten.
set -eu

here=$(cd "$(dirname "$0")" && pwd -P)
guard="$here/worktree-stray-guard.sh"
marker="worktree-stray-guard"

if [ "$#" -eq 0 ]; then
  umbrella=$(dirname "$(dirname "$here")")
  set -- "$umbrella"/foodshare-web "$umbrella"/foodshare-backend \
    "$umbrella"/foodshare-app "$umbrella"/foodshare-tools
fi

install_into() {
  dir=$1
  mkdir -p "$dir"
  for name in post-checkout pre-push; do
    target="$dir/$name"
    if [ -e "$target" ] && ! grep -q "$marker" "$target" 2>/dev/null; then
      echo "skip  $target (an unrelated $name hook is installed)" >&2
      continue
    fi
    cp "$guard" "$target"
    chmod 755 "$target"
    echo "ok    $target"
  done
}

for repo in "$@"; do
  git -C "$repo" rev-parse --git-dir >/dev/null 2>&1 || {
    echo "skip  $repo (not a git repository)" >&2
    continue
  }
  hooks=$(git -C "$repo" rev-parse --path-format=absolute --git-path hooks)
  if [ -n "$(git -C "$repo" config --get core.hooksPath || true)" ]; then
    echo "skip  $repo (core.hooksPath is set; install there by hand)" >&2
    continue
  fi
  install_into "$hooks"
done

template=$(git config --global --get init.templateDir || true)
if [ -n "$template" ]; then
  # A literal ~/ in the config value; git expands it, sh does not.
  # shellcheck disable=SC2088
  case "$template" in "~/"*) template="$HOME/${template#\~/}" ;; esac
  install_into "$template/hooks"
fi
