# C-094 WebDAV lock lookups fail closed

WebDAV mutating methods must never treat a lock lookup database error as "no locks found".

Required behavior:

- `LOCK`, `PUT`, `MKCOL`, `DELETE`, `MOVE`, and `COPY` conflict checks return locked/conflict behavior when active-lock lookup fails.
- Lock lookup errors are logged with request path context.
- Lock lookup errors increment an observable metric.
- Tests must cover the helper that converts lookup errors into conflicts.

Reason: a transient database error during lock conflict evaluation must not silently allow writes that bypass Class 2 locking.
