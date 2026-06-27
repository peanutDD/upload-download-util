# C-080 WebDAV Large Files Must Stream

WebDAV `PUT`, `GET`, `HEAD`, and Range responses must avoid whole-file buffering.

`PUT` writes request chunks to a temporary file while enforcing `storage.max_file_size`. `GET` and Range responses use the storage streaming APIs and set correct `Content-Length` / `Content-Range` headers.
