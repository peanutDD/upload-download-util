# C-081 WebDAV Writes Bump Cache

Every successful WebDAV write path must invalidate the user's file cache version.

This includes `PUT`, `MKCOL`, `DELETE`, `MOVE`, `COPY`, and future WebDAV methods that change file or folder visibility. Write paths must also preserve existing fulltext index enqueue/remove behavior.
