# Exec Plan: Codex Auto-Fix Bootstrap Hardening

## Goal

Stop self-hosted auto-review from looking stuck during PR-head bootstrap by making checkout local-first, budgeted, and visibly blocked when exact PR head cannot be verified.

## Assumptions

- The self-hosted runner can maintain a local bare mirror at `CODEX_LOCAL_BARE_MIRROR`.
- `CODEX_LOCAL_REPO_SEED` remains a useful fallback when the bare mirror is absent.
- GitHub tarball streaming remains transiently unreliable and must not consume the full review budget.
- Auto-fix must fail closed if exact PR head cannot be verified.

## Risks

- A stale local mirror may not contain the latest PR head. Mitigation: hydrate from both mirror and seed, then still require `git cat-file` for the exact SHA.
- A too-short tarball budget may require manual rerun during a GitHub outage. Mitigation: blocked comment explains bootstrap state and keeps automation from patching stale code.
- PR comments on bootstrap failure may duplicate if users rerun while GitHub is still unhealthy. Mitigation: comments are explicit operational evidence rather than silent hangs.

## Dependencies

- GitHub CLI can resolve `headRefName` and `headRefOid`.
- The local runner has read access to the configured mirror/seed paths.
- `GH_TOKEN` can post PR comments and read PR tarballs.

## Implementation

1. Add a failing workflow contract test for local bare mirror, local hydration, tarball budget, blocked status, and blocked PR comment.
2. Add `CODEX_PR_TARBALL_BUDGET_SECONDS` and `CODEX_LOCAL_BARE_MIRROR` to the workflow.
3. Refresh the local bare mirror with Git low-speed bounds when it exists.
4. Prefer cloning from the bare mirror, then the normal seed.
5. Hydrate missing PR head commits from local mirror/seed refs before any network tarball.
6. Replace long multiplicative tarball retry with bounded HTTP/1.1 curl attempts.
7. Log bootstrap status, elapsed time, bytes received, and post a `Codex checkout/bootstrap blocked` comment before fail-closed exit.

## Verification

- `cargo test --manifest-path scripts/codex-cli/Cargo.toml --test workflow_state codex_auto_fix_bootstraps_pr_head_without_git_https_checkout`
