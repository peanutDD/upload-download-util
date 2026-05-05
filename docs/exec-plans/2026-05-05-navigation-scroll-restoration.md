# Navigation Scroll Restoration Exec Plan

## Intent

Make Settings Back return to the previous page and preserve file-list scroll
across folder navigation, browser Back, and refresh.

## Assumptions

- Settings is normally opened from app history, preserving folder query params.
- File-list scroll is keyed by folder, sort, MIME filter, and search text.
- A page with no saved browsing position should still start at the top.

## Risks

- React Router navigation type alone is not enough: folder clicks are PUSH but
  may still have saved positions.
- Refresh/remount paths need current scroll persisted before unload.
- Existing theme work is in progress, so this task must avoid token/style churn.

## Dependencies

`frontend/src/pages/Settings.tsx`, `frontend/src/components/files/useFileList.ts`,
Vitest, React Router `MemoryRouter`, and sessionStorage-backed tests.

## Steps

1. Add failing Settings navigation regression for returning to
   `/files?folder=...` via history instead of hardcoded `/files`.
2. Add failing file-list hook regressions for restoring a saved folder scroll
   position on folder entry and persisting scroll on refresh/pagehide.
3. Change Settings button text to `Back` and invoke `navigate(-1)`.
4. Persist active file-list scroll on scroll/pagehide/visibility cleanup.
5. Restore saved scroll for any navigation mode; scroll top only when no saved
   position exists.
6. Run focused tests, related regression tests, lint, full test, and build.
