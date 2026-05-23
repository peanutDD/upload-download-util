# Exec Plan: Local Auto-Fix Kickoff Prefix Safety

Date: 2026-05-24

## Goal

Prevent local/Desktop Codex auto-fix commits from accidentally using the exact
GitHub Actions auto-fix commit prefix that `gemini-review-kickoff` skips.

## Assumptions

- The GitHub Actions `codex-auto-fix` workflow still owns the next Gemini review
  request after real workflow auto-fix commits.
- Local/Desktop Codex publish runs do not run `.github/scripts/codex-auto-fix-state.sh`
  and therefore do not automatically post the review status table.
- It is safer for local publishes to trigger the normal PR `synchronize`
  kickoff than to impersonate the workflow-owned auto-fix path.

## Risks

- Changing the exact workflow commit subject would cause duplicate Gemini
  review requests from `gemini-review-kickoff`.
- Leaving local publishes on the same prefix silently skips Gemini kickoff and
  hides the expected PR comment/status-table loop.
- Commit message behavior is shared by `pr-auto-fix`, local CLI, and ad-hoc
  Codex publish helpers, so the boundary needs explicit tests.

## Dependencies

- `scripts/codex-cli/src/repo.rs`
- `.github/workflows/codex-auto-fix.yml`
- `.github/workflows/gemini-review-kickoff.yml`
- `scripts/codex-cli/tests/workflow_state.rs`

## TDD Steps

1. Add failing tests proving local `commit_and_push_in(..., push=false)` uses
   `🤖 codex local auto-fix:` by default.
2. Add failing tests proving workflow-owned auto-fix can still opt into
   `🤖 codex auto-fix:`.
3. Add workflow contract coverage that the auto-fix workflow explicitly marks
   itself as review-owner, and that Gemini kickoff only skips the workflow prefix.
4. Implement the minimal commit message selector.
5. Update constraints and quality score after verification.

## Verification

- `cargo test --manifest-path scripts/codex-cli/Cargo.toml --lib repo::tests::commit_and_push_uses_local_prefix_by_default_so_gemini_kickoff_runs`
- `cargo test --manifest-path scripts/codex-cli/Cargo.toml --lib repo::tests::commit_and_push_keeps_workflow_prefix_when_workflow_owns_review`
- `cargo test --manifest-path scripts/codex-cli/Cargo.toml --test workflow_state -- --nocapture`
- `cargo fmt --all -- --check`
