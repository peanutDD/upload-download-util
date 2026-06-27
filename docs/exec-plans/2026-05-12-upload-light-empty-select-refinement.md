# Upload Light Empty Select Refinement Exec Plan

## Intent

Refine the light-mode empty upload dialog button styling after the initial
graphite/mint `Select files` treatment felt too heavy.

## Assumptions

- The main concern is the light-mode `Select files` action before any file is
  queued.
- Dark and purple empty-state palettes should remain unchanged.
- The light button should feel luminous and clean while preserving the upload
  dialog's tech-glass language.

## Risks

- Making the CTA too plain white can remove the app's visual character.
- Keeping too much green/graphite makes it look muddy against the light panel.

## Dependencies

- `frontend/src/components/files/upload/UploadDialog.css`
- `frontend/src/components/files/upload/UploadDialogLayout.test.ts`
- `docs/evidence/upload-empty-buttons-light.png`

## Steps

1. Add a failing test requiring a luminous light-mode select button palette.
2. Replace the light `Select files` gradient with a white/mint/cyan opal
   treatment and dark text.
3. Slightly soften light-mode cancel/attach and URL upload support colors.
4. Re-capture light visual evidence.
5. Run focused test, lint, build, and whitespace checks.
