# C-068: codex-cli provider-neutral review text input

`codex-auto-fix` must expose a provider-neutral text input flag for unstructured review content.

Required behavior:

- `pr-auto-fix` must accept `--review-text` as the preferred provider-neutral Markdown/text input.
- `pr-auto-fix` must continue accepting legacy `--gemini-review` for existing workflows.
- `pr-auto-fix` must reject simultaneous `--review-text` and `--gemini-review`.
- `auto-fix-local` must accept `--review-text` for inline local review content.
- `auto-fix-local` must reject simultaneous `--review-text` and `--review-file`.
- `--review-json` remains the preferred structured input and must not regress.

Rationale:

The tool is being gradually decoupled from Gemini-specific naming so it can consume GitHub Copilot Review, CodeRabbit, manual reviews, or custom review providers without changing the core auto-fix pipeline.
