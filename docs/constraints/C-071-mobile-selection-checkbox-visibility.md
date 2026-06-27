# C-071: Mobile Card Selection Controls Must Not Depend On Hover

Date: 2026-05-12

## Rule

File and folder card selection checkboxes must remain visible on coarse pointer
devices.

Do not use unconditional `invisible group-hover:visible`, opacity-only hover
reveal, or other hover-only disclosure for the primary selection control.

## Why

Touch devices do not have a reliable hover state. If an unselected checkbox is
hidden until `group:hover`, mobile users cannot discover or tap the selection
entry point.

## Required Pattern

- Default the card selection control to visible.
- Limit hover-only reveal behavior to `@media (hover: hover) and (pointer:
  fine)`.
- Keep selected checkboxes always visible on every input type.
- Cover this with a coarse pointer regression test that verifies unselected card
  selection controls are not rendered with unconditional `invisible`.
