# Codex Local Git Hooks

This project keeps Codex publish verification in one versioned script:

```bash
scripts/codex-cli/tools/pre-push-verify.sh --changed
```

Both GitHub `codex-auto-fix` automation and local `git push` hooks must call this script. Do not copy the backend/frontend command list into another workflow or hook.

## Install

Run once per local checkout:

```bash
bash scripts/git-hooks/install.sh
```

The installer copies the versioned template from:

```text
scripts/git-hooks/pre-push
```

to the untracked local Git hook path:

```text
.git/hooks/pre-push
```

## Behavior

- Backend changes run:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
- Frontend changes run:
  - `npm ci --ignore-scripts`
  - `npm run lint`
  - `npx --no-install tsc -b --noEmit`
- `CODEX_VERIFY_SCOPE=backend|frontend|all` can force a scope.
- `SKIP_CODEX_VERIFY=1 git push` or `git push --no-verify` is an emergency bypass only.

## Why Pre-Push

Rust format and clippy checks are too expensive for every commit. `pre-push` protects the remote branch while still allowing small local commits during development.

## Local Auto-Fix Commit Prefix

Local/Desktop Codex publishes use `🤖 codex local auto-fix:` by default. The
GitHub Actions `codex-auto-fix` workflow is the only path that opts into
`🤖 codex auto-fix:` with `CODEX_AUTO_FIX_WORKFLOW_OWNS_REVIEW=true`.

This keeps `gemini-review-kickoff` from skipping local publishes: local commits
trigger the normal PR `synchronize` review request unless they also implement
and test an equivalent PR comment/status-table path.
