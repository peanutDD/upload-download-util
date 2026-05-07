# Parallel Chunked Upload Exec Plan

Date: 2026-05-07

## Intent

Replace batch-barrier chunked upload scheduling with bounded N-way worker-pool scheduling so a completed chunk lane immediately starts the next pending part.

## Assumptions

- The backend chunk upload protocol remains unchanged.
- The frontend controls per-file chunk concurrency through `CHUNKED_UPLOAD.PARALLEL_CHUNKS`.
- Backend `upload_sessions.uploaded_parts` updates are already atomic enough for concurrent chunk requests.
- Existing resumable upload, cancellation, retry, and part SHA-256 semantics must remain unchanged.

## Risks

- Excessive parallel chunks can amplify browser connection pressure and backend disk writes.
- Concurrent progress updates can double count completed parts if client/server status overlaps.
- Failure handling must avoid completing an upload after one lane has already failed.

## Dependencies

- `frontend/src/services/fileUploadService.ts`
- `frontend/src/constants/index.ts`
- `frontend/src/services/fileUploadService.test.ts`
- `docs/constraints/C-018-upload-queue-cancel-must-settle.md`
- `docs/constraints/C-023-upload-resume-integrity.md`

## Steps

1. Add a RED test proving the next part starts as soon as one worker lane finishes.
2. Replace batch `Promise.allSettled(batch.map(uploadChunk))` scheduling with a bounded worker pool.
3. Track completed parts with a set to avoid duplicate progress increments.
4. Raise default chunk concurrency to 4 lanes.
5. Add a permanent constraint that chunked upload scheduling must not use batch barriers.
6. Run targeted frontend upload tests, lint, full frontend tests, and build.

## Verification

- `npm run test -- src/services/fileUploadService.test.ts`
- `npm run lint`
- `npm run test`
- `npm run build`
