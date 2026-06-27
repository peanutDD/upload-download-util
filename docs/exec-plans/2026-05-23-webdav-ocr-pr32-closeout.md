# WebDAV/OCR PR32 Closeout

## Intent

Make PR #32 merge-ready by hardening only WebDAV, fulltext, and OCR behavior on
`codex/webdav-fulltext-ocr-plan`.

## Assumptions

- The merge target is PR #32, not the dirty primary checkout.
- WebDAV remains available at `/dav` with Basic Auth using API tokens only.
- Fulltext and OCR public routes and environment variable names remain unchanged.
- Finder, iOS Files, rclone, and cadaver are release-candidate smoke checks, not
  merge blockers for this closeout.

## Risks

- WebDAV lock and path bugs can allow conflicting writes or invalid collection
  moves.
- Tantivy writer misuse can reintroduce lock contention or slow indexing.
- Fallback search must remain bounded and filename-only to avoid loading file
  bodies.
- OCR binary failures must be visible but must not fail worker processing.

## Dependencies

- Existing backend tests in `backend/tests/handler_webdav_tests.rs`.
- Existing fulltext/OCR tests in `backend/tests/fulltext_search_tests.rs`.
- Existing WebDAV smoke script at `scripts/webdav-smoke.sh`.
- Existing docs in `docs/references/fulltext-ocr-usage.md` and
  `docs/references/webdav-smoke.md`.

## Steps

1. Add focused failing tests for remaining WebDAV lock/path safety and bounded
   fulltext/OCR behavior.
2. Implement the smallest backend changes needed to pass those tests.
3. Update only closeout docs, constraints, and quality score entries tied to the
   verified behavior.
4. Run targeted backend tests, backend gates, and curl WebDAV smoke.
5. Update PR #32 notes so manual WebDAV clients are release-candidate checks.

