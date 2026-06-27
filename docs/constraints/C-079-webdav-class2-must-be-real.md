# C-079 WebDAV Class 2 Must Be Real

If `/dav` advertises `DAV: 1, 2` or includes `LOCK` / `UNLOCK` in `Allow`, the implementation must persist locks and enforce `If` / `Lock-Token` on mutating methods.

Never return synthetic lock tokens without conflict checks. If lock persistence is unavailable, downgrade the WebDAV response to Class 1 only.
