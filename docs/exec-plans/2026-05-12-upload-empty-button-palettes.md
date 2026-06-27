# Upload Empty Button Palettes Exec Plan

## Intent

Redesign the upload dialog button colors across dark, light, and purple modes
before any local file has been selected.

## Assumptions

- The empty state is `uploadFiles.length === 0`, surfaced as
  `data-ready-to-upload="false"` on the footer.
- `Select files` is the primary empty-state local action.
- Footer `Attach files` remains disabled until files are queued, but should look
  intentionally inactive rather than like a broken primary button.
- URL `Upload` belongs to the remote URL panel and needs a separate secondary
  action treatment.

## Risks

- Reusing the generic primary button class for all upload actions makes future
  color changes bleed across unrelated buttons.
- Disabled states can become too bright and look clickable.
- Theme-specific hover and disabled declarations can silently fail if they use
  undefined CSS tokens.

## Dependencies

- `frontend/src/components/files/upload/UploadDialog.tsx`
- `frontend/src/components/files/upload/UploadDropzone.tsx`
- `frontend/src/components/files/upload/UrlUploadForm.tsx`
- `frontend/src/components/files/upload/UploadDialog.css`
- `frontend/src/components/files/upload/UploadDialogLayout.test.ts`

## Steps

1. Add failing tests requiring semantic button classes and empty-state palette
   tokens.
2. Add semantic classes for cancel, attach, select files, and URL upload.
3. Define dark, light, and purple empty-state palette variables in
   `UploadDialog.css`.
4. Apply scoped empty-state footer styles and standalone select/URL styles.
5. Capture visual evidence for dark, light, and purple empty states.
6. Run focused test, lint, build, and whitespace checks.
