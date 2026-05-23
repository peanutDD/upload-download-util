# WebDAV/OCR Shared Verify Rerun Exec Plan

Date: 2026-05-24

## Goal

Re-run the WebDAV/OCR closeout on the isolated `codex/webdav-fulltext-ocr-plan` worktree and publish the remaining automation closeout changes through the Codex auto-fix commit path:

```text
Codex auto-fix publish
  -> CODEX_AUTO_FIX_VERIFY_COMMANDS
  -> scripts/codex-cli/tools/pre-push-verify.sh --changed
  -> commit/push only after verification succeeds
```

Also confirm the human local path remains active:

```text
git push
  -> .git/hooks/pre-push
  -> scripts/codex-cli/tools/pre-push-verify.sh --changed
  -> push only after verification succeeds
```

## Scope

- Use only the isolated worktree:
  `/Users/tyone/.config/superpowers/worktrees/upload-download-util/codex/webdav-ocr-closeout`
- Preserve unrelated dirty changes in the primary checkout.
- Re-run WebDAV/OCR/backend evidence from the PR #32 closeout.
- Publish only the shared verify/hook closeout files and documentation.

## Assumptions

- WebDAV/OCR feature code is already in `e09fe5f` on `codex/webdav-fulltext-ocr-plan`.
- Remaining uncommitted files are automation/hook closeout files created after the first closeout.
- `codex-auto-fix` has no public "publish current tree" subcommand, so the rerun will invoke the same production commit function, `codex_cli::repo::commit_and_push_in`, from a temporary external runner.

## Risks

- The shared verifier detects changed `backend/` or `frontend/` paths only. This rerun still executes targeted WebDAV/OCR/backend gates before publish because this is a WebDAV/OCR closeout.
- The local primary checkout is dirty; no reset, checkout, or clean command may be used there.
- Push can be blocked by remote/network state. If blocked, leave local commit and report the exact error.

## Verification Commands

- `cargo test --manifest-path scripts/codex-cli/Cargo.toml --test workflow_state`
- `bash -n scripts/codex-cli/tools/pre-push-verify.sh`
- `bash -n scripts/git-hooks/pre-push`
- `bash -n scripts/git-hooks/install.sh`
- `backend: cargo test --test handler_webdav_tests -- --nocapture`
- `backend: cargo test --test fulltext_search_tests -- --nocapture`
- `backend: cargo test --test handler_files_upload_tests -- --nocapture`
- `backend: cargo test --test service_auth_tests -- --nocapture`
- `backend: cargo fmt --all -- --check`
- `backend: cargo check`
- `backend: cargo clippy --all-targets --all-features -- -D warnings`
- `backend: cargo test --all-features`
- `CODEX_AUTO_FIX_VERIFY_COMMANDS='bash scripts/codex-cli/tools/pre-push-verify.sh --changed'` around the Codex auto-fix publish call.

## Publish File List

- `.github/workflows/codex-auto-fix.yml`
- `docs/constraints/C-084-codex-auto-fix-pre-push-validation.md`
- `docs/quality-score.md`
- `docs/exec-plans/2026-05-23-codex-shared-pre-push-verify.md`
- `docs/exec-plans/2026-05-24-webdav-ocr-shared-verify-rerun.md`
- `docs/references/codex-local-hooks.md`
- `scripts/codex-cli/tests/workflow_state.rs`
- `scripts/codex-cli/tools/pre-push-verify.sh`
- `scripts/git-hooks/pre-push`
- `scripts/git-hooks/install.sh`
