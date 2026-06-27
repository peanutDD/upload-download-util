# C-083 WebDAV PROPFIND XML Must Use A Parser

WebDAV `PROPFIND` request bodies must be parsed as XML before selecting requested properties.

Do not infer requested live props with substring search or regex. Malformed XML returns `400 Bad Request`, comments and unrelated text do not request properties, and unknown requested properties stay in the `207 Multi-Status` response as `404 propstat` entries.
