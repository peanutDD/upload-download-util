# Trash Card Action Row Exec Plan

## Intent

Redesign trash page file cards so restore and purge actions live on a dedicated
button row beneath the title, details, and retention text.

## Assumptions

- Only the trash page card layout changes.
- Trash APIs, batch toolbar actions, theme tokens, and backend behavior remain
  unchanged.
- Card titles stay single-line with ellipsis.
- Buttons keep stopping propagation so card selection is not toggled by action
  clicks.
- Existing glass button styling and lucide icons remain the visual language.

## Risks

- The trash grid can render very narrow cards, so action labels must not crowd
  the card or force unstable heights.
- Moving actions out of the thumbnail must preserve restore and purge behavior.
- Fluid sizing constraints must be preserved for user-visible dimensions.
- The current worktree contains unrelated changes, so this task must avoid
  broad formatting or unrelated files.

## Dependencies

- `frontend/src/pages/Trash.tsx`
- `frontend/src/pages/Trash.test.tsx`
- `frontend/src/components/files/list/FileListGlass.css`
- `docs/constraints/C-020-card-hover-spacing.md`
- `docs/constraints/C-021-card-title-single-line.md`
- `docs/constraints/C-030-fluid-sizing-governance.md`

## Steps

1. Add a failing trash card test that expects a dedicated action row.
2. Move restore and purge buttons from the thumbnail overlay into the card meta
   area beneath the retention countdown.
3. Add scoped card action row/button styling that preserves narrow-grid
   readability and fluid sizing.
4. Run the focused trash page test.
5. Run frontend fluid sizing checks and lint if the focused test passes.
6. Review the final diff to confirm only the intended trash layout files changed.
