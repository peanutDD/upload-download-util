# Unselected Card Checkbox Contrast Exec Plan

## Intent

Improve the contrast of unselected file/folder card checkboxes so the outer
circle and inner ring remain visible against thumbnail backgrounds.

## Assumptions

- The checkbox visibility issue is already fixed by C-071.
- The remaining problem is visual contrast, not DOM presence or click handling.
- `SelectionCheckbox` is shared by file, folder, and trash cards, so one
  component-level styling change covers the card surfaces.

## Risks

- Over-brightening the unselected checkbox could make it compete with selected
  state.
- Hardcoded colors would bypass theme-specific contrast needs.

## Dependencies

- `frontend/src/components/common/form/SelectionCheckbox.tsx`
- `frontend/src/components/files/list/FileListGlass.css`
- `frontend/src/styles/tokens.css`
- `frontend/src/components/files/grid/FileCardMobileDragMove.test.tsx`

## Steps

1. Add a failing regression test requiring high-contrast unselected checkbox
   classes.
2. Add unselected outer and ring semantic classes in `SelectionCheckbox`.
3. Add theme tokens for unselected surface, border, ring, and shadow.
4. Style the unselected classes in `FileListGlass.css`.
5. Add permanent constraint and quality score entry.
6. Run focused tests, full frontend tests, lint, token checks, build, visual
   evidence, and diff whitespace checks.
