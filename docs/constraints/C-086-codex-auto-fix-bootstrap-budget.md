# C-086: Codex Auto-Fix bootstrap must be local-first and budgeted

## Rule

The self-hosted `codex-auto-fix` bootstrap must prefer local PR-head sources before any GitHub tarball download:

- use `CODEX_LOCAL_BARE_MIRROR` when a maintained local bare mirror exists,
- refresh that mirror with bounded Git low-speed settings before workspace bootstrap,
- fall back to `CODEX_LOCAL_REPO_SEED`,
- hydrate missing commits from local refs before network fallback,
- bound all PR-head tarball acquisition by `CODEX_PR_TARBALL_BUDGET_SECONDS`,
- log bootstrap phase/status with elapsed time and downloaded bytes,
- post a clear `Codex checkout/bootstrap blocked` PR comment before failing closed when exact PR head cannot be verified.

The workflow must not replace this with unbounded or multiplicative retries.

## Why

GitHub API/codeload tarball streams can repeatedly fail with HTTP/2 cancellation, timeout, or connection resets. Retrying for many minutes before review execution makes the automation appear stuck and burns the same budget needed for actual review repair.

Exact PR-head verification remains mandatory. The safer fix is to make local sources the normal path, keep network fallback under a hard budget, and expose bootstrap failures as bootstrap failures instead of review failures.

## Enforcement

`scripts/codex-cli/tests/workflow_state.rs::codex_auto_fix_bootstraps_pr_head_without_git_https_checkout` asserts local bare mirror support, local hydration, bounded tarball retry budget, explicit blocked status, and the PR blocked comment.
