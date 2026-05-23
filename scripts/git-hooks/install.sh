#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

hook_src="scripts/git-hooks/pre-push"
# Installs into the untracked local Git hook path, normally .git/hooks/pre-push.
hook_dst="$(git rev-parse --git-path hooks/pre-push)"

if [[ ! -f "$hook_src" ]]; then
  echo "Missing hook template: $hook_src" >&2
  exit 1
fi

mkdir -p "$(dirname "$hook_dst")"

if [[ -f "$hook_dst" ]] && ! cmp -s "$hook_src" "$hook_dst"; then
  backup="${hook_dst}.backup.$(date +%Y%m%d%H%M%S)"
  cp "$hook_dst" "$backup"
  echo "Backed up existing pre-push hook to $backup"
fi

cp "$hook_src" "$hook_dst"
chmod +x "$hook_dst"

echo "Installed Codex pre-push hook at $hook_dst"
