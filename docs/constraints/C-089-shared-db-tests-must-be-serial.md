# C-089 Shared Database Tests Must Be Serial

Tests that call `cleanup_test_data` against the shared PostgreSQL test database must not run concurrently with other tests that create users, folders, files, API tokens, WebDAV locks, or background tasks in that same database.

Required behavior:

- Add `serial_test::serial` to integration tests that clean shared database tables and then assert on inserted rows.
- Use a stable group name per test file or domain, for example `#[serial(service_auth_db)]`, so cleanup cannot delete another test's just-created fixtures.
- Prefer isolated per-test schemas or databases for future broad rewrites; until then, do not rely on default Rust test parallelism for shared DB integration tests.

This prevents foreign-key and missing-row flakes where one async test deletes shared fixtures while another is between insert and assertion.
