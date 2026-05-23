# C-095 WebDAV collection copy/move cannot target descendants

WebDAV collection `COPY` and `MOVE` must reject destinations that are the source collection itself or any descendant path.

Required behavior:

- `COPY /dav/a` with `Destination: /dav/a/b` returns `409 Conflict`.
- `MOVE /dav/a` with `Destination: /dav/a/b` returns `409 Conflict`.
- The rejection must happen before destination overwrite/delete side effects.
- File copy/move behavior is unchanged.

Reason: copying or moving a collection into itself can create recursive structures, partial writes, or destructive overwrite behavior that different clients handle poorly.
