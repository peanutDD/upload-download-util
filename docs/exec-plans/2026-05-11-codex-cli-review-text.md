# Exec Plan: codex-cli Review Text Alias

Date: 2026-05-11

## Goal

Decouple the CLI input contract from Gemini-specific naming by adding `--review-text` while keeping `--gemini-review` compatible.

## Assumptions

- Existing GitHub Actions may still pass `--gemini-review`, so the old flag must keep working.
- `--review-json` remains the preferred structured input path.
- `--review-text` is plain Markdown/text content, not a file path.

## Risks

- Accepting both `--review-text` and `--gemini-review` could make the effective input ambiguous.
- Changing the old flag would break the Markdown rollback path in `.github/workflows/codex-auto-fix.yml`.
- This is only a CLI contract step; deeper Gemini provider naming remains a separate follow-up.

## Steps

1. Add failing e2e tests for `auto-fix-local --review-text` and ambiguous alias rejection.
2. Add `--review-text` to `pr-auto-fix` and `auto-fix-local`.
3. Reject simultaneous `--review-text` and legacy text/file aliases.
4. Update CLI and development docs.
5. Record the permanent input-contract constraint (`C-068`) and quality score.
6. Run fmt, tests, clippy, reinstall, and doctor.

## Verification

- `cargo test --manifest-path scripts/codex-cli/Cargo.toml --test e2e_auto_fix review_text`
- `cargo fmt --manifest-path scripts/codex-cli/Cargo.toml -- --check`
- `cargo test --manifest-path scripts/codex-cli/Cargo.toml`
- `cargo clippy --manifest-path scripts/codex-cli/Cargo.toml --all-targets -- -D warnings`
- `cargo install --path scripts/codex-cli --force`
- `codex-auto-fix doctor --json`
