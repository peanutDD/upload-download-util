# Settings Visual Polish Execution Plan

Date: 2026-05-15

## Goal

Improve the Settings page visual quality without changing the current content
layout or section order.

## Assumptions

- The approved layout stays intact: Settings Center header first, then Account,
  Security, WebDAV Access, API Tokens, OCR Status, and Storage.
- This is a presentation-only pass; form behavior, WebDAV guidance content, API
  token behavior, OCR status, and storage data contracts stay unchanged.
- The design should feel more deliberate through subtle surfaces, highlights,
  input focus, and button feedback rather than another structural redesign.

## Risks

- Too much decoration could make the settings page noisier instead of calmer.
- Hover/focus effects must not shift layout or make controls harder to scan.
- Visual changes need desktop and mobile screenshots because dense Settings
  forms can overflow on narrow screens.

## Steps

1. Add a focused regression test that checks the visual shell hooks while
   preserving the current Settings section order.
2. Polish the shared `SettingsCard` shell with grouped hover state, subtle
   hairlines, and improved icon treatment.
3. Polish shared input, button, and panel helpers in `settingsUi`.
4. Add matching depth and hairline treatment to the Settings Center header and
   KPI tiles.
5. Verify with focused Settings tests, lint, production build, and desktop /
   mobile production-preview screenshots.

## Acceptance

- The six Settings content cards remain in the approved order.
- Visual polish is applied through shared components rather than one-off
  section rewrites.
- Focused Settings regression tests pass.
- Lint and production build pass.
- Production-preview screenshots show no error boundary and no detected text or
  control overflow on desktop or mobile.
