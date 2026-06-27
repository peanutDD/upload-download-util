# C-084 Codex Auto Fix Must Validate Before Push

Codex Auto Fix must not publish generated patches before running the configured repository verification commands.

Required guard:

- `CODEX_AUTO_FIX_VERIFY_COMMANDS` runs inside `codex-auto-fix` before `git add`, `git commit`, or `git push`.
- `CODEX_AUTO_FIX_VERIFY_COMMANDS` must delegate to `scripts/codex-cli/tools/pre-push-verify.sh --changed`; local git hooks must call the same script instead of duplicating validation commands.
- The versioned local hook template lives at `scripts/git-hooks/pre-push`; install it into the untracked `.git/hooks/pre-push` path with `scripts/git-hooks/install.sh`.
- A non-zero verification exit blocks the auto-fix commit, returns structured JSON with `push_blocked=true`, and leaves the PR for human inspection instead of failing the workflow before the state machine can run.
- Frontend changes must run fail-fast dependency install, lint, and TypeScript checks before publish so generated imports cannot introduce undeclared packages.
- Self-hosted runner frontend verification must use `npm ci --ignore-scripts` and `npx --no-install tsc` so native optional build steps such as `canvas` cannot hide TypeScript validation behind local Node/toolchain drift.
- Backend changes must run `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D warnings` before publish because GitHub API fallback pushes do not reliably trigger CI on the new commit.

Regression test:

- `cargo test --manifest-path scripts/codex-cli/Cargo.toml codex_auto_fix_runs_frontend_pre_push_validation`
- `cargo test --manifest-path scripts/codex-cli/Cargo.toml codex_auto_fix_uses_shared_pre_push_verify_script`
- `cargo test --manifest-path scripts/codex-cli/Cargo.toml codex_shared_pre_push_verify_script_covers_backend_and_frontend`
- `cargo test --manifest-path scripts/codex-cli/Cargo.toml codex_git_hook_template_and_installer_delegate_to_shared_verify_script`
- `cargo test --manifest-path scripts/codex-cli/Cargo.toml commit_and_push_runs_configured_verify_commands_before_commit`
- `cargo test --manifest-path scripts/codex-cli/Cargo.toml auto_fix_local_reports_push_blocked_when_pre_push_validation_fails`
- `cargo test --manifest-path scripts/codex-cli/Cargo.toml codex_auto_fix_runs_backend_pre_push_format_validation`
