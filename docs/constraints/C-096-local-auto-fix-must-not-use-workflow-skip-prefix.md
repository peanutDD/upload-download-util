# C-096: Local Auto-Fix Must Not Use Workflow Skip Prefix

Local/Desktop Codex auto-fix publishes must not use the exact commit subject
prefix that `gemini-review-kickoff` skips.

Rules:

- Only the GitHub Actions `codex-auto-fix` workflow may opt into
  `🤖 codex auto-fix:` by setting `CODEX_AUTO_FIX_WORKFLOW_OWNS_REVIEW=true`.
- Local/Desktop publishes use `🤖 codex local auto-fix:` by default.
- `gemini-review-kickoff` must skip only the exact workflow-owned prefix, never
  the local prefix.
- Any local publish path that wants to suppress kickoff must first post an
  equivalent PR status table/comment and make that behavior explicit in tests.

Why:

The workflow-owned auto-fix commit prefix means the state machine will request
or clear the next Gemini review itself. A local publish does not run that state
machine, so using the same prefix makes PR `synchronize` skip Gemini kickoff and
can leave reviewers without the expected review status table.

Coverage:

- `scripts/codex-cli/src/repo.rs` tests local and workflow commit subjects.
- `scripts/codex-cli/tests/workflow_state.rs` locks the workflow opt-in and
  kickoff skip boundary.
