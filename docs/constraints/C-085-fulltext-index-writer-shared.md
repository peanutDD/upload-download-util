# C-085 Fulltext Index Writer Must Be Shared Per AppState

Tantivy allows only one writer lock per index directory. Runtime code and tests must use the `AppState.search_index` instance for indexing/searching instead of opening a second `SearchIndexService` on the same `SEARCH_INDEX_PATH`.

Opening a second writer on an active directory can produce `LockBusy` and make search workers fail nondeterministically.
