# C-070: Codex Auto-Fix review runs must not stale-block the queue

## Rule

`Codex Auto Fix (本地 Runner)` must use workflow-level concurrency so same-PR `codex-fix` runs are serialized before they contend for the self-hosted runner. Active runs must not be canceled by newer review events, because cancellation can interrupt commit publishing or review state advancement after useful fixes have already been generated.

Required guards:

- workflow-level concurrency with `cancel-in-progress: false`
- concurrency group must be keyed by PR number: `codex-auto-fix-pr-<pr-number>`. Do not key it by `github.ref`/`github.actor`, because `issue_comment` review events can collapse unrelated PRs onto the default branch and make healthy runs wait behind another PR.
- job-level `timeout-minutes`
- `调用 codex-auto-fix pr-auto-fix` step-level `timeout-minutes`
- `CODEX_AGENT_TIMEOUT_SECONDS` lower than the step timeout
- `CODEX_AUTO_FIX_BUDGET_SECONDS` lower than the step timeout so `codex-cli` can stop starting new model calls and return JSON before GitHub Actions kills the shell
- the workflow must run `codex-auto-fix doctor --json` after resolving `CODEX_AGENT_COMMAND` and before starting `pr-auto-fix`; `agent.command=warning` must fail fast instead of entering a recursive local runner call
- model-based post-fix checks such as `SecurityCheck` and `QualityScore` are soft gates: timeout or parse failure must be recorded as pending/unavailable output, not process failure
- queue guard diagnostics that print the PR concurrency group and timeout budget

## Why

Gemini can submit multiple `pull_request_review` events for the same PR head. The workflow needs a single PR-number concurrency group so the self-hosted runner is not asked to execute multiple auto-fix jobs for the same review stream at once. However, newer events must wait instead of canceling the active job.

`Waiting: This job is waiting on codex-fix ... to complete before running.`

That queued state is acceptable when one auto-fix is already running for the same PR. It is not acceptable for unrelated PRs to wait behind a default-branch `issue_comment` group. A canceled active run can fail after pushing or while advancing the review state machine, which makes the PR look broken even though the generated fix commit exists. Job-level concurrency is too late for self-hosted runner assignment and can still leave newer jobs waiting. Timeouts remain as a second guard for the run that is actually executing.

Another hard stall mode is `CODEX_AGENT_COMMAND` pointing back to `codex-auto-fix` instead of the real Codex CLI executor. That creates recursive runner calls and looks like an infinite local auto-fix run. `doctor --json` is the preflight guard for this configuration error.

The workflow must not rely only on an outer Actions timeout. In PR #32 the `调用 codex-auto-fix pr-auto-fix` step was killed after 30 minutes while `QualityScore` was running. The CLI had already applied useful patches, but the shell timeout prevented JSON output, commit, push, and state advancement. The runtime budget must therefore be visible to `codex-cli`, and optional LLM audits must degrade gracefully.

## Enforcement

`scripts/codex-cli/tests/workflow_state.rs::codex_auto_fix_serial_runner_has_timeouts` locks the timeout and diagnostics contract.
`scripts/codex-cli/tests/workflow_state.rs::codex_auto_fix_concurrency_serializes_without_canceling_active_runs` locks workflow-level serialization without active-run cancellation.
`scripts/codex-cli/tests/workflow_state.rs::codex_auto_fix_runs_doctor_before_long_auto_fix_step` locks the doctor preflight before the expensive auto-fix command.
`scripts/codex-cli/tests/workflow_state.rs::codex_auto_fix_has_coherent_runtime_budget` locks the job, step, model-call, and CLI budget relationship.
`scripts/codex-cli/tests/doctor.rs::doctor_warns_when_agent_command_recurses_into_auto_fix_binary` locks recursive `CODEX_AGENT_COMMAND` detection.
`scripts/codex-cli/tests/e2e_auto_fix.rs::auto_fix_local_keeps_partial_fix_when_security_check_times_out` and `auto_fix_local_keeps_partial_fix_when_quality_score_times_out` lock graceful degradation for slow optional checks.
