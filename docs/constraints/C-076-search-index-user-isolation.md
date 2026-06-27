# C-076 Fulltext Index Results Must Be User-Isolated

Fulltext search indexes may contain documents from multiple users, but search results must be filtered by authenticated `user_id` before returning data.

Required behavior:

- Search documents include `file_id`, `user_id`, `filename`, `path`, text fields, category, and MIME type.
- Query results must only include the current user.
- Deleted files must be removed from the index or excluded by fallback/API logic.
- Index repair/reindex tasks must use dedupe key `search:<file_id>` for single-file work.

This prevents cross-user metadata or content leakage through shared local Tantivy indexes.
