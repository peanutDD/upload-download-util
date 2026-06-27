# Upload Purple Ready Button Hover Exec Plan

## Intent

Apply the reference bid/artwork button treatment to the purple-theme upload
dialog footer when files are ready to upload.

## Assumptions

- `Attach files` / `Start Upload` maps to the filled purple gradient hover.
- `Cancel` maps to the dark outlined purple hover.
- The treatment only applies in purple mode and only when the upload queue has
  files.
- Empty queue, light/dark themes, and disabled states keep their existing
  behavior.

## Risks

- A hover-only CSS change can regress silently if scoped selectors drift.
- Invalid theme token references can cause browsers to drop the hover
  declaration.
- The repo has unrelated dirty work, so the change must stay scoped to upload
  dialog files and task docs.

## Dependencies

- `frontend/src/components/files/upload/UploadDialog.tsx`
- `frontend/src/components/files/upload/UploadDialog.css`
- `frontend/src/components/files/upload/UploadDialogLayout.test.ts`
- `frontend/src/styles/tokens.css`

## Steps

1. Add a failing regression test for purple ready-state hover selectors.
2. Add a ready-state data attribute to the upload dialog footer.
3. Add purple-only hover CSS for the primary and secondary footer buttons.
4. Verify hover declarations use defined theme tokens.
5. Capture visual evidence for both hover states.
6. Run focused test, lint, and build.
