# C-065: codex-cli local install freshness

When `scripts/codex-cli` source changes affect CLI behavior, local users must be able to verify whether the binary on PATH is stale.

Required behavior:

- `codex-auto-fix doctor` must report the running binary path, the source manifest directory, PATH resolution for `codex-auto-fix` and `codex`, and a reinstall hint.
- `codex-auto-fix doctor --json` must emit parseable JSON for automation.
- Doctor must warn when the running binary appears older than source files.
- Doctor must warn when `CODEX_AGENT_COMMAND` is missing, but it must not write secrets, mutate `.env`, or change repository settings.
- After source updates, refresh local usage with:

```bash
cargo install --path scripts/codex-cli --force
```

Rationale:

On 2026-05-11 the repository source and debug binary included recent commands such as `review-to-json` and `auto-fix-weekly-report`, but `/Users/tyone/.cargo/bin/codex-auto-fix` still pointed to a 2026-05-03 install. GitHub Actions was safe because it used `cargo run --manifest-path`, while local terminal usage remained stale.
