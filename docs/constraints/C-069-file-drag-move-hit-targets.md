# C-069: File Drag Move Hit Targets Must Match User-Visible Cards

Date: 2026-05-11

## Rule

Dragging a file onto a folder must work when the release point is anywhere on
the visible folder card, not only on the folder icon/thumb area.

Mobile file drag must also start from the visible thumbnail surface, even when a
hidden hover-only preview action is layered above the thumbnail.

## Why

Desktop users naturally release on the folder card label, padding, or edge. If
drop handlers live only on the icon/thumb child, those releases silently fail.

Touch users often long-press the center of a file thumbnail. A transparent
hover-only preview button can still be the event target in the DOM and must not
block the long-press drag gesture.

## Required Pattern

- Put desktop folder `dragover` and `drop` handlers on the element carrying
  `data-folder-id`.
- Keep nested draggable folder icon/thumb responsible for folder drag start.
- Mark hover-only preview controls so mobile long-press startup can ignore them
  as blockers while preserving normal preview click behavior.
- Keep hover-only overlays `pointer-events: none` until their hover state enables
  desktop interaction.
- Disable native HTML `draggable` on coarse pointer devices. Desktop mouse drag
  can use HTML5 drag/drop, but touch long-press must stay on the custom pointer
  event path.
- Prevent native `contextmenu`/long-press callout while a touch drag is pending,
  otherwise the user may only feel system haptic feedback and never enter the
  app's drag state.
- Register a global mobile drag finish fallback from pointer-down until
  pointer-up/cancel so the move still completes when touch release is delivered
  outside the source card.
- Cover both desktop full-card drop and mobile hidden-preview long-press with
  regression tests, including coarse-pointer `draggable=false` and native
  callout suppression.
