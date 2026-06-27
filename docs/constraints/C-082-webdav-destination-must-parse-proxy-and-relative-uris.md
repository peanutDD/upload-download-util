# C-082 WebDAV Destination Must Parse Proxy And Relative URIs

WebDAV `Destination` parsing must support absolute URLs, reverse-proxy prefixes before `/dav`, URL-encoded path segments, absolute DAV paths, and client-supplied relative paths.

Do not parse `Destination` with string splits tied to a fixed host or deployment root. Relative destinations are resolved against the source resource collection before the normalized path is passed through the normal path sanitizer.
