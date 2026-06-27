# WebDAV Permission Controls Execution Plan

Date: 2026-05-15

## Goal

Replace the default checkbox appearance for `Enable WebDAV` and `WebDAV
read-only` with polished permission cards that match the Settings page visual
language while preserving checkbox behavior.

## Assumptions

- Only the API Tokens WebDAV permission controls change visually.
- `react-hook-form` registration, checkbox field names, and token creation
  payloads stay unchanged.
- The controls remain keyboard-focusable and label-clickable through native
  checkbox semantics.

## Risks

- Fully replacing checkbox visuals can accidentally break accessibility if the
  input is removed instead of visually hidden.
- Checked-state styling must be supported by the current Tailwind/Vite build.

## Steps

1. Keep the native checkbox inputs and convert them to `sr-only peer` controls.
2. Redesign each option as a full clickable permission card with a custom
   rounded check mark, selected-state background, hover state, and helper copy.
3. Add regression coverage that the custom cards still submit the expected
   WebDAV payload.
4. Run focused Settings tests, lint, build, and production-preview screenshot
   verification.

## Acceptance

- The browser default checkbox square is no longer visible.
- `Enable WebDAV` defaults checked and `WebDAV read-only` defaults unchecked.
- Clicking `WebDAV read-only` still submits `webdav_read_only: true`.
- Focused test, lint, build, and screenshot verification pass.
