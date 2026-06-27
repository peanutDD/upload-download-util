# WebDAV Smoke Matrix

Run these against a local or preview deployment after backend checks pass.

- Automated curl smoke: `WEBDAV_URL=http://127.0.0.1:3000/dav WEBDAV_TOKEN=... scripts/webdav-smoke.sh`.
- `rclone`: configure WebDAV vendor `other`, run `mkdir`, `copy`, `ls`, `cat`, `moveto`, `delete`.
- `cadaver`: connect to `/dav`, run `mkcol`, `put`, `propfind`, `lock`, `unlock`, `move`, `delete`.
- macOS Finder: Connect to Server with `/dav`, upload a file, rename it, duplicate it, delete it, verify `.DS_Store` does not appear in the web UI.
- iOS Files: Connect with Basic Auth, upload an image, preview/download it from the web UI, delete it from Files and verify it enters Trash.

Expected protocol invariants:

- `OPTIONS /dav` advertises `DAV: 1, 2`.
- Locked resources reject writes without a matching `If` or `Lock-Token`.
- Lock conflict lookup/storage errors fail closed instead of allowing writes through.
- Collection `COPY`/`MOVE` into the source collection or any descendant returns `409 Conflict`.
- Large downloads and Range requests stream from storage.
- API tokens marked read-only cannot perform mutating WebDAV methods.

## 2026-05-14 Codex Local Smoke

Automated curl smoke passed against `http://127.0.0.1:3017/dav` using a freshly created WebDAV-enabled API token.

Evidence directory: `docs/evidence/webdav-smoke-20260514-codex/`

Captured evidence:

- `options.txt`: `OPTIONS /dav` advertised `DAV: 1, 2` and `LOCK, UNLOCK`.
- `propfind-depth-1.xml`: `Depth: 1` returned the collection and uploaded file in a `207 Multi-Status` body.
- `range.txt`: suffix Range returned `206 Partial Content`, `Content-Range: bytes 13-18/19`, and the expected body.
- `lock-response.txt`: `LOCK` returned a persisted `opaquelocktoken`.
- `locked-write-status.txt`: write without the lock token returned `423 Locked`; write with `If: (<token>)` then succeeded.

Local client availability:

- `curl`: available and passed.
- `rclone`: not installed in this environment.
- `cadaver`: not installed in this environment.
- macOS Finder and iOS Files: not exercised from this headless Codex run; still require manual UI smoke on a reachable build before final release signoff.

## 2026-05-23 PR #32 Closeout Smoke

Automated curl smoke passed against `http://127.0.0.1:3019/dav` using a freshly created WebDAV-enabled read/write API token.

Evidence directory: `docs/evidence/webdav-smoke-20260523-pr32-closeout/`

Captured evidence:

- `options.txt`: `OPTIONS /dav` advertised `DAV: 1, 2` and `LOCK, UNLOCK`.
- `propfind-depth-1.xml`: `Depth: 1` returned the smoke collection and uploaded file.
- `range.txt`: suffix Range returned `206 Partial Content`, `Content-Range: bytes 13-18/19`, and the expected `smoke` body.
- `lock-response.txt`: `LOCK` returned a persisted `opaquelocktoken`.
- `locked-write-status.txt`: write without the lock token returned `423 Locked`; write with `If: (<token>)` then succeeded.

Local client availability:

- `curl`: available and passed.
- `rclone`: not installed in this environment.
- `cadaver`: not installed in this environment.
- macOS Finder and iOS Files: release-candidate smoke items, not PR #32 merge blockers.

## 2026-05-24 Shared Verify Re-run Smoke

Automated curl smoke passed against `http://127.0.0.1:3024/dav` using a freshly created WebDAV-enabled read/write API token.

Evidence directory: `docs/evidence/webdav-smoke-20260524-shared-verify/`

Captured evidence:

- `options.txt`: `OPTIONS /dav` advertised `DAV: 1, 2` and `LOCK, UNLOCK`.
- `propfind-depth-1.xml`: `Depth: 1` returned the smoke collection and uploaded file.
- `range.txt`: suffix Range returned `206 Partial Content`, `Content-Range: bytes 13-18/19`, and the expected `smoke` body.
- `lock-response.txt`: `LOCK` returned a persisted `opaquelocktoken`.
- `locked-write-status.txt`: write without the lock token returned `423 Locked`; write with `If: (<token>)` then succeeded.

Local client availability:

- `curl`: available and passed.
- `rclone`: not installed in this environment.
- `cadaver`: not installed in this environment.
- macOS Finder and iOS Files: release-candidate smoke items, not PR #32 merge blockers.
