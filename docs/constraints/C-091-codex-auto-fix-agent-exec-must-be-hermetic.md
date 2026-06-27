# C-091: Codex Auto-Fix agent exec must be hermetic and process-group bounded

## Rule

`codex-auto-fix` model calls must not inherit interactive Codex app behavior that can load plugins, MCP servers, or remote plugin indexes.

For commands whose executable is `codex` and subcommand is `exec`, `scripts/codex-cli` must inject these non-interactive defaults before the prompt argument:

- `--ignore-user-config`
- `--ignore-rules`
- `--ephemeral`
- `--model ${CODEX_AGENT_MODEL:-gpt-5.5}`

Every local agent command must also run in its own process group. On timeout, `codex-auto-fix` must terminate the whole process group, not only the direct child, so grandchildren such as `git remote-https` or MCP helper processes cannot keep the runner stuck.

## Why

The 2026-05-15 self-hosted run reached `codex-cli` and then stalled inside `/Applications/Codex.app/.../codex exec --skip-git-repo-check -`. Process inspection showed Codex had spawned `git remote-https https://github.com/openai/plugins.git` and a Computer Use MCP process before producing the repair output. That made review repair depend on interactive plugin startup instead of the actual GPT-5.5 text repair task.

## Enforcement

`scripts/codex-cli/src/llm.rs` has unit coverage for:

- injecting hermetic Codex exec defaults without duplicating explicit flags,
- preserving explicit model overrides,
- killing a spawned grandchild process when the model command times out.

`scripts/codex-cli/tests/workflow_state.rs` asserts the workflow passes `CODEX_AGENT_MODEL: gpt-5.5`.
