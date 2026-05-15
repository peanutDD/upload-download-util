# C-093: Relaxed Review Clear Must Post Status Table

When `CODEX_AUTO_FIX_STRICT=false`, clearing Gemini review state must still make
the latest review findings visible in the PR comments.

Required behavior:

- `pr-auto-fix` JSON output must be persisted for the state-machine step.
- The `relaxed_clear` branch must post a fresh Markdown table for the current
  Gemini review with: severity, location, Gemini issue, Codex status, and
  solution/explanation.
- The comment must state that relaxed mode means the findings no longer block
  the automated loop, not that every finding was necessarily fixed in code.
- If structured result JSON is unavailable or invalid, the state comment must
  explicitly say the table could not be generated instead of silently pointing
  to older comments.

This keeps PR reviewers from mistaking a green relaxed automation status for an
empty or fully fixed Gemini review.
