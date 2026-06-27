# C-075 WebDAV Basic Auth Uses API Tokens Only

WebDAV Basic authentication must treat the password field as an API Token, never as an account login password.

Required behavior:

- `Authorization: Basic base64(username:api_token)` verifies through `ApiTokenService`.
- `username` is not trusted for authorization; the token determines `user_id`.
- Account passwords must fail WebDAV authentication.
- Logs and metrics must not record raw tokens.

This keeps WebDAV clients compatible with Basic Auth while preserving the API Token revocation and expiry boundary.
