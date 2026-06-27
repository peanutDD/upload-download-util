# C-070: Filter Dropdowns Must Stay Anchored To Their Toolbar

Date: 2026-05-11

## Rule

File list filter dropdowns, including Type and Sort, must be rendered inside the
filter toolbar DOM tree and positioned relative to their trigger card.

They must not be portaled to `document.body` or positioned with viewport-fixed
coordinates.

## Why

The file toolbar scrolls with the page. A body-level fixed dropdown stays pinned
to the viewport when the user scrolls, so it appears to float away from the
Type/Sort controls instead of moving with the toolbar.

The dropdown panel also needs an intentionally solid surface. Reusing the
semi-transparent filter-card background makes the options look washed out and
harder to scan.

## Required Pattern

- Keep dropdown menu DOM under the filter card that opened it.
- Use `position: absolute` anchored to the trigger card, not `position: fixed`.
- Keep the open trigger card above sibling cards so the menu is not covered.
- If the toolbar uses `transform` on an inner scale wrapper, raise that toolbar
  wrapper above file content with a toolbar-level z-index token. Raising only the
  dropdown panel is insufficient because transform creates a new stacking
  context.
- Use dropdown-specific surface tokens with stronger opacity than the trigger
  filter card.
- Cover this with a regression test that asserts opened menus remain inside the
  rendered filter container, CSS does not reintroduce fixed positioning, and the
  transformed toolbar stays above file content while menus are open.
