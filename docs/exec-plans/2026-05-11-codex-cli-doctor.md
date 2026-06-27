# Exec Plan: codex-cli Doctor and Local Install Refresh

Date: 2026-05-11

## Goal

Make `codex-cli` report whether the binary on PATH is stale after source updates, then refresh the local install.

## Assumptions

- GitHub Actions already runs `codex-auto-fix` through `cargo run --manifest-path scripts/codex-cli/Cargo.toml`, so CI is not blocked by a stale local install.
- The stale path issue affects local terminal usage through `/Users/tyone/.cargo/bin/codex-auto-fix` and `/Users/tyone/.cargo/bin/codex`.
- A diagnostic command is safer than relying on manual timestamp checks.

## Risks

- `cargo install --force` overwrites the local `codex` and `codex-auto-fix` binaries.
- `CODEX_AGENT_COMMAND` may be intentionally unset in a development shell; doctor should warn, not mutate secrets or `.env`.
- PATH can resolve to a different binary than the one currently running; doctor should report this clearly.

## Steps

1. Add a failing integration test for `codex-auto-fix doctor --json`.
2. Implement a reusable `doctor` module with JSON and human-readable output.
3. Wire the `doctor` subcommand into both `codex` and `codex-auto-fix` bins.
4. Document the command, reinstall flow, and permanent freshness constraint.
5. Reinstall the local CLI with `cargo install --path scripts/codex-cli --force`.
6. Verify with fmt, test, clippy, and `codex-auto-fix doctor`.

## Verification

- `cargo test --manifest-path scripts/codex-cli/Cargo.toml --test doctor`
- `cargo fmt --manifest-path scripts/codex-cli/Cargo.toml -- --check`
- `cargo test --manifest-path scripts/codex-cli/Cargo.toml`
- `cargo clippy --manifest-path scripts/codex-cli/Cargo.toml --all-targets -- -D warnings`
- `cargo install --path scripts/codex-cli --force`
- `codex-auto-fix doctor --json`
