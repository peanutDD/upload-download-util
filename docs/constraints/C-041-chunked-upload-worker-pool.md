# C-041: Chunked upload concurrency must use a bounded worker pool

Date: 2026-05-07

## Rule

Chunked upload concurrency must be implemented as a bounded worker pool, not as fixed-size batches with a barrier between batches.

## Why

Fixed-size batches waste available upload lanes when one chunk is slow. A completed lane must immediately claim the next pending part so large uploads keep steady throughput without exceeding the configured concurrency limit.

## Required Pattern

- Use `CHUNKED_UPLOAD.PARALLEL_CHUNKS` as the upper bound for per-file chunk workers.
- Start at most that many chunk requests at once.
- When one chunk resolves, the same worker should immediately start another pending chunk.
- Track completed part numbers with a set so retries, resume status checks, and concurrent updates cannot double count progress.
- Preserve abort, retry, resume, and `X-Part-SHA256` behavior from C-018 and C-023.
