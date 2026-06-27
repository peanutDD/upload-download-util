# Codex Auto-Fix Relaxed Repair

## Goal

Prioritize repairing Gemini Review findings with Codex/GPT-5.5 instead of stopping on recoverable patch, context, retry, or soft-audit failures.

## Assumptions

- `CODEX_AGENT_COMMAND` points to the real Codex/GPT-5.5 executor.
- `context_mismatch`, empty retry patches, and retry-generation failures can still be repaired through full-file fallback.
- SecurityCheck and QualityScore are soft review aids; they should not override the user's goal of fixing review findings.

## Risks

- Full-file fallback can produce larger diffs than SEARCH/REPLACE.
- Relaxed workflow mode must still surface soft audit findings in comments/JSON.
- Pre-push validation must remain hard, otherwise broken code could be published.

## Plan

1. Add red e2e coverage for `context_mismatch` where retry generation returns empty output and full-file fallback must still fix the file.
2. Add workflow-state coverage for relaxed pending behavior.
3. Continue into full-file fallback after empty initial patch, empty retry patch, retry-generation failure, and patch context drift.
4. Run the workflow with `CODEX_AUTO_FIX_STRICT=false`.
5. Keep protected files, checkout exactness, write boundaries, and pre-push validation as hard stops.

## Verification

- `cargo test --manifest-path scripts/codex-cli/Cargo.toml --test e2e_auto_fix context_mismatch_retry_empty_still_uses_full_file_fallback -- --nocapture`
- `cargo test --manifest-path scripts/codex-cli/Cargo.toml --test workflow_state relaxed_pending_without_fix_keeps_review_loop_running -- --nocapture`
- `cargo fmt --manifest-path scripts/codex-cli/Cargo.toml -- --check`
- `cargo test --manifest-path scripts/codex-cli/Cargo.toml`
