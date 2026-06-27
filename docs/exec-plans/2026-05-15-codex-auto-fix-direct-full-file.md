# 2026-05-15 Codex Auto-Fix Direct Full-File Repair

## Exec Plan

Goal: stop relaxed BatchFix from being blocked by patch context mismatch.

Assumptions:

- Relaxed GitHub auto-fix should prioritize getting Gemini review findings fixed and published.
- Protected files, allowed write prefixes, checkout correctness, and pre-push validation remain hard boundaries.
- Patch/retry behavior remains available for local or strict legacy coverage.

Risks:

- Full-file replacement has a larger edit surface than SEARCH/REPLACE.
- Model output that is actually a patch must still be rejected before writing source.
- Pre-push validation must remain the final publish gate.

Steps:

1. Add `CODEX_AUTO_FIX_DIRECT_FULL_FILE` support in BatchFix.
2. Enable that mode in the relaxed workflow.
3. Add an E2E regression proving direct mode fixes without patch-apply retry logs.
4. Update permanent constraints and quality score.
5. Run codex-cli formatting, tests, clippy, and workflow script syntax checks.
