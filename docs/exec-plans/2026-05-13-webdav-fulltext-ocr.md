# WebDAV + Fulltext/OCR Execution Plan

Date: 2026-05-13

## Goal

Implement WebDAV access at `/dav` and fulltext search with a Tantivy index in `/Users/tyone/github/upload-download-util`, reusing the existing API token, storage, file, folder, background task, and content extraction layers.

## Assumptions

- This run intentionally works in the primary checkout because the human explicitly requested no isolated worktree.
- WebDAV v1 maps only the authenticated user's private file tree.
- Basic auth accepts `username:api_token`; account passwords are rejected.
- OCR remains optional and config-gated. No `leptess`/Tesseract system dependency is introduced in this phase.
- Tantivy is local single-writer storage; distributed consistency is documented as a v1 constraint.

## Risks

- The primary checkout already has unrelated dirty changes. This task only edits WebDAV/fulltext/settings/docs files.
- Full WebDAV protocol compatibility is broad. v1 covers Finder/rclone basics and returns explicit errors for unsupported methods.
- Fulltext indexing can lag behind upload/delete mutations; API fallback and task dedupe reduce user-visible gaps.

## Dependencies

- Rust crates: `base64`, `tantivy`.
- Existing Postgres `background_tasks` table and API token verification.
- Existing `StorageBackend`, `FileService`, `FolderService`, and `FileContentExtractor`.

## Steps

1. Add WebDAV auth/router tests, then implement `/dav` handler.
2. Add Tantivy service tests, then implement `SearchIndexService`.
3. Add fulltext API and background task tests, then implement handler and workers.
4. Add settings page WebDAV guidance and regression test.
5. Add constraints and update quality score.
6. Verify backend and frontend commands.

## Acceptance

- WebDAV supports `OPTIONS`, `PROPFIND`, `MKCOL`, `PUT`, `GET`, `HEAD`, `DELETE`, `MOVE`; unsupported `COPY`, `LOCK`, `UNLOCK` return explicit 501.
- Fulltext search filters by `user_id`, excludes deleted files, supports snippets, and falls back safely when the index is not ready.
- Upload/delete/restore enqueue `search_index_file` or `search_remove_file` with dedupe key `search:<file_id>`.
- Settings page documents WebDAV URL and API token usage without exposing any token.
