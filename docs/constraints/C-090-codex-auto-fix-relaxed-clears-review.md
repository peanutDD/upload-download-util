# C-090: Relaxed Codex Auto-Fix clears Gemini review state immediately

## Rule

When `CODEX_AUTO_FIX_STRICT=false`, `Codex Auto Fix (本地 Runner)` is an automatic repair helper, not a strict merge gate.

In relaxed mode, after `codex-fix` processes a Gemini review:

- use direct full-file repair (`CODEX_AUTO_FIX_DIRECT_FULL_FILE=true`) so patch context drift does not block GPT-5.5 from writing the fix,
- do not request another Gemini review,
- do not wait on Gemini watchdog,
- do not leave `gemini-review-needs-human` or `gemini-review-pending`,
- clear review loop labels and mark `gemini-review-round-max` + `gemini-review-clean`,
- treat push-blocked, soft audit findings, or remaining pending explanations as non-blocking automation output.

Strict fail-closed behavior is available only when `CODEX_AUTO_FIX_STRICT=true`.

`gemini-review-kickoff` must also be non-blocking in relaxed mode:

- keep posting `/gemini review`,
- do not fail the check if Gemini does not respond before the watchdog timeout,
- do not add `gemini-review-needs-human` for a missing Gemini response unless `GEMINI_REVIEW_REQUIRED=true`.

## Why

The project owner wants Gemini findings to be consumed by Codex quickly without turning every external service delay, patch fallback, SecurityCheck warning, or GitHub API hiccup into a blocking review state. The automation should repair what it can, publish/record the result, and clear the automatic review loop so human work can continue.

## Enforcement

`scripts/codex-cli/tests/workflow_state.rs` asserts that relaxed pending and relaxed push-blocked states both plan `relaxed_clear`, mark the review clean, and do not request another Gemini review.
`scripts/codex-cli/tests/e2e_auto_fix.rs` asserts direct full-file mode fixes the target file without emitting a patch-apply failure retry.
`scripts/codex-cli/tests/gemini_watchdog.rs` asserts missing Gemini responses are non-blocking by default and strict timeout behavior is opt-in.
