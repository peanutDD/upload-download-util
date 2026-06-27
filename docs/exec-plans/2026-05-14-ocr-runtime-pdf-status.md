# OCR Runtime, PDF OCR, and Status UI Execution Plan

Date: 2026-05-14

## Goal

Close the remaining OCR deliverable gaps for `/Users/tyone/github/upload-download-util`: Docker/runtime Tesseract dependencies, scanned PDF page-level OCR, and a frontend OCR status management surface.

## Assumptions

- OCR remains config gated outside Docker Compose; `OCR_ENABLED=false` stays the default app config.
- Docker Compose may enable OCR because the container image now installs the required runtime dependencies.
- PDF OCR v1 uses Poppler `pdftoppm` page rendering, bounded by `OCR_PDF_MAX_PAGES`; layout-aware scanned PDF extraction is still future work.
- Missing Tesseract or Poppler must be observable and non-fatal.

## Risks

- OCR system dependencies increase image size.
- Running OCR on many scanned pages can be CPU-heavy, so the page limit must remain explicit.
- Status UI must not expose secrets or worker internals.

## Dependencies

- Runtime binaries: `tesseract`, `pdftoppm`.
- Existing fulltext worker and `SearchIndexService`.
- Existing Settings page card system and authenticated API client.

## Steps

1. Add failing backend tests for Docker OCR packages, PDF page conversion, dependency-missing fallback, and OCR status endpoint.
2. Add failing frontend test for Settings OCR status readiness.
3. Implement OCR config for `OCR_PDFTOPPM_BIN` and `OCR_PDF_MAX_PAGES`.
4. Implement PDF OCR through `pdftoppm` plus per-page Tesseract extraction.
5. Add authenticated OCR status API and Settings status card.
6. Update Docker image/Compose env, constraints, quality score, and run focused plus build verification.

## Acceptance

- Backend tests prove Dockerfile contains Tesseract and Poppler runtime packages.
- PDF OCR renders only the configured page count and indexes text from each rendered page.
- Missing Poppler/Tesseract returns dependency-missing/skipped status instead of failing the worker.
- Settings displays OCR enabled state, Tesseract readiness, Poppler readiness, and PDF page limit.
