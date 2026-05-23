#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: pre-push-verify.sh [--changed|--all]

Runs the repository checks that must pass before Codex auto-fix or a local
pre-push hook publishes changes.

Environment:
  CODEX_VERIFY_SCOPE=auto|backend|frontend|all  Override changed-file detection.
  SKIP_CODEX_VERIFY=1                           Explicit emergency bypass.
USAGE
}

mode="--changed"
if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
elif [[ "${1:-}" == "--all" || "${1:-}" == "--changed" ]]; then
  mode="$1"
elif [[ $# -gt 0 ]]; then
  usage >&2
  exit 2
fi

if [[ "${SKIP_CODEX_VERIFY:-}" == "1" ]]; then
  echo "SKIP_CODEX_VERIFY=1: skipping Codex verification"
  exit 0
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

frontend_changed=false
backend_changed=false

mark_from_worktree_status() {
  # Keep these exact probes aligned with the existing codex-auto-fix workflow
  # contract tests and the pre-commit publish guard.
  if git status --short -- frontend/ | grep -q .; then
    frontend_changed=true
  fi
  if git status --short -- backend/ | grep -q .; then
    backend_changed=true
  fi
}

mark_from_upstream_diff() {
  local upstream
  upstream="$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null || true)"
  if [[ -z "$upstream" ]]; then
    return
  fi

  if ! git diff --quiet "$upstream"...HEAD -- frontend/; then
    frontend_changed=true
  fi
  if ! git diff --quiet "$upstream"...HEAD -- backend/; then
    backend_changed=true
  fi
}

mark_from_pre_push_stdin() {
  local local_ref local_sha remote_ref remote_sha
  local zero_sha="0000000000000000000000000000000000000000"

  while read -r local_ref local_sha remote_ref remote_sha; do
    [[ -z "${local_ref:-}" || "$local_sha" == "$zero_sha" ]] && continue

    if [[ "$remote_sha" == "$zero_sha" ]]; then
      if git diff-tree --no-commit-id --name-only -r "$local_sha" -- frontend/ | grep -q .; then
        frontend_changed=true
      fi
      if git diff-tree --no-commit-id --name-only -r "$local_sha" -- backend/ | grep -q .; then
        backend_changed=true
      fi
    else
      if git diff --name-only "$remote_sha..$local_sha" -- frontend/ | grep -q .; then
        frontend_changed=true
      fi
      if git diff --name-only "$remote_sha..$local_sha" -- backend/ | grep -q .; then
        backend_changed=true
      fi
    fi
  done
}

case "${CODEX_VERIFY_SCOPE:-auto}" in
  all)
    frontend_changed=true
    backend_changed=true
    ;;
  frontend)
    frontend_changed=true
    backend_changed=false
    ;;
  backend)
    frontend_changed=false
    backend_changed=true
    ;;
  auto)
    if [[ "$mode" == "--all" ]]; then
      frontend_changed=true
      backend_changed=true
    else
      mark_from_worktree_status
      mark_from_upstream_diff
      if [[ ! -t 0 ]]; then
        mark_from_pre_push_stdin
      fi
    fi
    ;;
  *)
    echo "Unsupported CODEX_VERIFY_SCOPE=${CODEX_VERIFY_SCOPE}" >&2
    exit 2
    ;;
esac

if [[ "$frontend_changed" == "true" ]]; then
  echo "Codex verify: frontend changed; running npm install/lint/typecheck"
  (
    cd frontend
    npm ci --ignore-scripts
    npm run lint
    npx --no-install tsc -b --noEmit
  )
else
  echo "Codex verify: frontend unchanged; skipping"
fi

if [[ "$backend_changed" == "true" ]]; then
  echo "Codex verify: backend changed; running cargo fmt/clippy"
  (
    cd backend
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
  )
else
  echo "Codex verify: backend unchanged; skipping"
fi

echo "Codex verification completed"
