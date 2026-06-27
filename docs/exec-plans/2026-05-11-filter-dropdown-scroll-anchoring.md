# Filter Dropdown Scroll Anchoring Exec Plan

## Intent

Fix Type and Sort dropdowns so they scroll with the file toolbar instead of
floating at a viewport-fixed position, and make the dropdown panel less
transparent.

## Assumptions

- The visible issue is caused by `DropdownMenu` portaling menu content to
  `document.body` with fixed positioning.
- The follow-up cover issue is caused by the scaled toolbar wrapper creating a
  transform stacking context below the file content.
- The file toolbar can allow overflow for the scaled filter row.
- Dropdown panels should remain visually related to the trigger cards but use a
  stronger, more readable surface.

## Risks

- Moving the dropdown back into the toolbar can reintroduce clipping if toolbar
  overflow is hidden.
- Type and Sort cards can overlap each other unless the open card gets a higher
  stacking order.
- Raising only the dropdown panel can still fail if its transformed toolbar
  parent remains under later file-content layers.
- Light and dark themes need separate solid-enough dropdown surfaces.

## Dependencies

- `frontend/src/components/common/DropdownMenu.tsx`
- `frontend/src/components/files/list/FileListFilters.css`
- `frontend/src/components/files/list/FileListGlass.css`
- `frontend/src/components/files/list/FileListFilters.test.tsx`
- `frontend/src/styles/tokens.css`

## Steps

1. Add failing regression tests for in-container dropdown rendering and non-fixed
   CSS.
2. Remove `createPortal` and fixed viewport positioning from `DropdownMenu`.
3. Anchor the dropdown panel absolutely to the trigger card and raise the open
   card above siblings.
4. Raise the transformed toolbar wrapper above file content so the in-container
   dropdown is not covered.
5. Add dropdown-specific stronger surface tokens for dark and light themes.
6. Run focused filter tests, full frontend tests, lint, and build.
7. Add permanent constraint and quality score entry.
