# C-088 WebDAV API Layer Must Stay Thin

WebDAV HTTP handlers may parse requests, map errors to HTTP responses, emit protocol headers, and record request metrics. Business operations that touch storage, database rows, cache invalidation, fulltext indexing, or recursive file/folder tree changes must live behind `services::webdav::WebDavService`.

Required behavior:

- Keep `/dav` route entrypoints stable through `backend/src/api/webdav/mod.rs`.
- Keep HTTP method body/header handling in `backend/src/api/webdav/methods.rs`, PROPFIND/XML in `backend/src/api/webdav/propfind.rs`, and Class 2 lock protocol handling in `backend/src/api/webdav/lock.rs`.
- Put protocol-only helpers in `backend/src/api/webdav/path.rs`, `range.rs`, `lock_headers.rs`, and `xml_fragments.rs`.
- Put resource lookup, copy/move/delete tree behavior, cache bumps, and fulltext enqueue behavior in `backend/src/services/webdav.rs`.
- Preserve WebDAV Class 2 lock enforcement and cache/fulltext side effects when moving code across the boundary.

This keeps WebDAV protocol parsing reviewable without hiding persistence side effects inside a large API file.
