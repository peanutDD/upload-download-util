# C-087 OCR Runtime, PDF Page Limit, and Status Must Stay Observable

OCR support must keep all three layers in sync:

- container/runtime images include both `tesseract-ocr` and `poppler-utils`;
- scanned PDF OCR goes through `pdftoppm` with a bounded `OCR_PDF_MAX_PAGES` page limit before Tesseract runs;
- Settings can query `/api/v1/files/search/ocr/status` to show whether OCR is enabled and whether Tesseract/Poppler are available.

Missing OCR dependencies must degrade to a skipped/dependency-missing status. They must not fail uploads, fulltext search, or the worker loop.
