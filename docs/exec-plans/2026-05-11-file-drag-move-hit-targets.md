# File Drag Move Hit Targets Exec Plan

## Intent

Fix file drag-to-folder movement on desktop and mobile by aligning drag/drop hit
targets with the visible card surfaces users actually touch or release on.

## Assumptions

- The shared `handleDropOnFolder` action layer and folder move API are already
  correct.
- The failure is in UI hit testing before the shared action layer is called.
- Desktop users may release on any visible part of a folder card.
- Mobile users may long-press the thumbnail center where a hover-only preview
  control exists in the DOM.
- Mobile release can be delivered outside the source card on some browser/WebView
  pointer paths.
- Native HTML `draggable` on touch browsers can preempt the custom long-press
  pointer path and surface only system haptic feedback.

## Risks

- Moving desktop drop handlers must not break folder drag start.
- Mobile long-press must still ignore menus, checkboxes, and visible controls.
- Hover preview clicks must keep working on desktop.
- Existing selected-item batch drag semantics must stay unchanged.
- Global pointer listeners must clean themselves up on pointerup, pointercancel,
  short tap, movement cancel, and unmount.
- Desktop mouse drag/drop must continue using native HTML drag.

## Dependencies

- `frontend/src/components/files/grid/FileCard.tsx`
- `frontend/src/components/files/grid/FolderCard.tsx`
- `frontend/src/components/files/grid/FileCardMobileDragMove.test.tsx`
- `frontend/src/components/files/grid/FolderCardDragMove.test.tsx`
- `frontend/src/hooks/files/useFileActions.ts`

## Steps

1. Add failing tests for desktop full-folder-card drop and mobile hidden-preview
   long-press startup.
2. Move folder card drop handling to the full `data-folder-id` card surface.
3. Mark hover-only preview action and keep its overlay non-interactive until
   desktop hover.
4. Add a global mobile file drag finish fallback for pointerup/pointercancel
   delivered outside the source card.
5. Disable native HTML draggable on coarse pointers and suppress native
   contextmenu/callout while touch drag is pending.
6. Verify focused grid tests and existing drag-move action/list regressions.
7. Add a permanent constraint and quality score entry.
