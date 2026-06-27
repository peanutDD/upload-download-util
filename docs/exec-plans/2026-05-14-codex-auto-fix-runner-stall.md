# Exec Plan: Codex Auto-Fix Runner Stall Closure

## Goal

Stop `Codex Auto Fix (本地 Runner)` / `codex-fix` runs from appearing stuck when they are triggered by `pull_request_review` or Gemini review comments.

## Assumptions

- Same-PR review events must remain serialized because concurrent auto-fix pushes can corrupt the review loop.
- Different PRs must not block each other behind a default-branch `issue_comment` concurrency group.
- `CODEX_AGENT_COMMAND` must point to the real Codex CLI executor, not `codex-auto-fix` itself.
- Existing job, step, agent, and CLI budget timeouts remain valid.

## Risks

- Canceling an active run can interrupt commit publish or state-machine advancement.
- Keying concurrency by `github.ref` can serialize unrelated PRs when review comments run on the default branch.
- A recursive `CODEX_AGENT_COMMAND` looks like a hung local runner unless it fails before `pr-auto-fix`.

## Implementation

1. Change workflow-level concurrency to `codex-auto-fix-pr-<pr-number>` with `cancel-in-progress: false`.
2. Add a workflow doctor preflight after resolving `CODEX_AGENT_COMMAND` and before `pr-auto-fix`.
3. Teach `doctor` to warn when `CODEX_AGENT_COMMAND` recursively points to `codex-auto-fix`.
4. Update C-070 and CLI docs with the new operational rule.
5. Lock the behavior with `workflow_state` and `doctor` tests.

## Verification

- `cargo test --manifest-path scripts/codex-cli/Cargo.toml --test workflow_state codex_auto_fix`
- `cargo test --manifest-path scripts/codex-cli/Cargo.toml --test doctor doctor_warns_when_agent_command_recurses_into_auto_fix_binary`
