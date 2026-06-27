# C-097: WebDAV Empty Responses Must Report Zero Content-Length

WebDAV `GET` and `HEAD` responses for zero-byte files must set
`Content-Length: 0`.

Rules:

- Use the already computed response payload length for `Content-Length`.
- Do not derive `Content-Length` from `end - start + 1` unless the file is
  non-empty and the range is valid.
- Zero-byte file responses must have an empty body and `Content-Length: 0`.
- Invalid Range requests against zero-byte files may remain
  `416 Range Not Satisfiable`, but successful empty-file `GET`/`HEAD` must not
  advertise one byte.

Why:

For `total == 0`, `end` is initialized with `total.saturating_sub(1)`, which is
`0`. Recomputing `end.saturating_sub(start) + 1` then produces `1` even though
the response body is empty. Some WebDAV clients can hang, retry, or mark the
transfer corrupt when `Content-Length` does not match the actual body.

Coverage:

- `backend/tests/handler_webdav_tests.rs::webdav_empty_file_get_and_head_report_zero_content_length`
