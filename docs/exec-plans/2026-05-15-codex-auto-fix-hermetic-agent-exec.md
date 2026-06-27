# 2026-05-15 Codex Auto-Fix Hermetic Agent Exec

## Exec Plan

Goal: prevent self-hosted auto-fix from stalling inside interactive Codex app plugin or MCP initialization.

Assumptions:

- The live stall is caused by the local Codex executor inheriting user config and spawning plugin/MCP helper processes.
- Auto-fix model calls only need deterministic GPT-5.5 text output.
- Timeouts must kill grandchildren, not just the direct `codex` process.

Risks:

- Ignoring user config means the workflow must pass the intended model explicitly.
- Process-group termination is Unix-specific.
- A currently running old workflow will keep using the old binary until cancelled or timed out.

Steps:

1. Reproduce the timeout leak with a child process that spawns a grandchild.
2. Put local agent commands in their own process group and kill the process group on timeout.
3. Normalize `codex exec` commands with `--ignore-user-config`, `--ignore-rules`, `--ephemeral`, and `--model gpt-5.5`.
4. Pass `CODEX_AGENT_MODEL` through the workflow.
5. Add permanent constraint and quality score records.
6. Run codex-cli tests, clippy, formatting, and script syntax checks.
