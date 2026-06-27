# Mobile Selection Checkbox Visibility Exec Plan

## Intent

Fix the mobile regression where the top-left selection checkbox on file/folder
cards is missing unless a hover state exists.

## Assumptions

- The checkbox DOM is still rendered by `FileCard` and `FolderCard`.
- The invisible state is caused by hover-only reveal classes in
  `SelectionCheckbox`.
- File and folder cards share the same checkbox component, so fixing the common
  visibility rule covers both card types.

## Risks

- Making the checkbox always visible globally could change desktop hover
  behavior.
- A component-level fix without CSS media scoping could leave future hover-only
  regressions hard to detect.

## Dependencies

- `frontend/src/components/common/form/SelectionCheckbox.tsx`
- `frontend/src/components/files/list/FileListGlass.css`
- `frontend/src/components/files/grid/FileCardMobileDragMove.test.tsx`

## Steps

1. Add a failing coarse-pointer regression test for unselected checkbox
   visibility.
2. Replace unconditional `invisible group-hover:visible` with a semantic reveal
   class.
3. Scope hover-only hiding to `@media (hover: hover) and (pointer: fine)`.
4. Add permanent constraint and quality score entry.
5. Run focused tests, full frontend tests, lint, token checks, build, and diff
   whitespace checks.
