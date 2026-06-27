# Codex Shared Pre-Push Verify Hook Exec Plan

Date: 2026-05-23

## Goal

Make Codex auto-fix and local human pushes use the same repository verification gate so backend/frontend validation runs automatically before generated or manual patches are published.

## Scope

- Add one versioned verification script as the single source of truth.
- Wire `CODEX_AUTO_FIX_VERIFY_COMMANDS` to that script.
- Add a versioned `pre-push` hook template and installer.
- Install the hook in the local project after the versioned files are in place.
- Keep this change limited to codex automation, git hook tooling, docs, and tests.

## Assumptions

- The existing dirty primary checkout contains unrelated work and must not be cleaned or reverted.
- The isolated worktree at `/Users/tyone/.config/superpowers/worktrees/upload-download-util/codex/webdav-ocr-closeout` is the merge-ready implementation source.
- A local hook can be installed once into `.git/hooks/pre-push`; `.git/hooks` itself remains untracked.
- `pre-push` should be meaningful but not exhaustive; full CI remains the final gate.

## Risks

- Hooks that are too slow encourage bypassing. The default hook runs existing format/lint/type gates only for changed backend/frontend paths.
- Local dependency drift can make hooks fail differently from CI. The frontend check uses `npm ci --ignore-scripts` and `npx --no-install` to keep behavior close to the current automation.
- Worktree and primary checkout can drift. This task explicitly syncs the new versioned files into both locations.

## Dependencies

- Bash
- Git
- Rust toolchain for backend checks
- Node/npm for frontend checks
- Existing `scripts/codex-cli` Rust test suite

## TDD Plan

1. Add failing contract tests that require:
   - `.github/workflows/codex-auto-fix.yml` to call the shared verify script from `CODEX_AUTO_FIX_VERIFY_COMMANDS`.
   - The shared verify script to exist and contain backend/frontend gates.
   - The versioned `pre-push` hook and installer to exist and delegate to the shared verify script.
2. Watch the targeted tests fail.
3. Implement the scripts and workflow wiring.
4. Re-run targeted tests and shell syntax checks.

## Verification

- `cargo test --manifest-path scripts/codex-cli/Cargo.toml codex_auto_fix_uses_shared_pre_push_verify_script`
- `cargo test --manifest-path scripts/codex-cli/Cargo.toml codex_shared_pre_push_verify_script_covers_backend_and_frontend`
- `cargo test --manifest-path scripts/codex-cli/Cargo.toml codex_git_hook_template_and_installer_delegate_to_shared_verify_script`
- `bash -n scripts/codex-cli/tools/pre-push-verify.sh`
- `bash -n scripts/git-hooks/pre-push`
- `bash -n scripts/git-hooks/install.sh`
- `bash scripts/git-hooks/install.sh` in the local primary checkout

## Rollback

- Versioned files can be reverted with a normal git revert.
- Local hook can be removed with `rm .git/hooks/pre-push` if needed.
