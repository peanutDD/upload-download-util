# C-092: Codex Auto Fix Reused Workspace Must Be Cleaned

Self-hosted `codex-fix` runners reuse `GITHUB_WORKSPACE`, so bootstrap must
clean any existing checkout before hydrating and checking out a PR head.

Required behavior:

- After opening an existing workspace checkout and before local mirror/seed
  hydration, run `git reset --hard` and `git clean -ffdx`.
- Never clean the local seed repository itself. If `GITHUB_WORKSPACE` resolves
  to the same path as `CODEX_LOCAL_REPO_SEED`, stop with
  `bootstrap_status=blocked reason=workspace_matches_seed`.
- Stale local edits in the runner workspace are bootstrap residue, not review
  findings. They must not make `git checkout -B <PR_HEAD>` fail with
  "local changes would be overwritten by checkout".

This prevents a previous auto-fix run from blocking the next review bootstrap
when files were refactored, deleted, or moved between commits.
