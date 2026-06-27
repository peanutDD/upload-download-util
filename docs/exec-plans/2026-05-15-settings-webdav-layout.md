# Settings WebDAV Layout Execution Plan

Date: 2026-05-15

## Goal

Remove the Settings Appearance section from the page and reorganize WebDAV Access so connection details, credentials, setup steps, and client notes are easy to scan.

## Assumptions

- Appearance means the Settings page `ThemeSection` block, not the standalone theme component tests.
- WebDAV behavior, URL shape, authentication model, and token creation flow stay unchanged.
- Existing unrelated worktree changes are user-owned and must not be reverted.

## Risks

- Settings regression tests already cover navigation and WebDAV guidance, so assertions must be updated without weakening existing behavior.
- More WebDAV copy can make the section feel busier unless it is grouped into clear panels.
- Removing the Appearance block from Settings should not remove the theme store or theme component itself.

## Dependencies

- `frontend/src/pages/Settings.tsx`
- `frontend/src/components/settings/WebDavAccessSection.tsx`
- `frontend/src/components/settings/SettingsPageRegression.test.tsx`
- Existing Settings card and semantic theme tokens.

## Steps

1. Add failing Settings regressions for no Appearance section and the new WebDAV structure.
2. Remove `ThemeSection` from the Settings page render path.
3. Rebuild `WebDavAccessSection` into endpoint, credential, setup-step, and client-note groups.
4. Run focused frontend tests, then run lint/build checks as time allows.
5. Update quality score after verification.
6. Align the WebDAV desktop grid as equal-height paired rows while keeping mobile order readable.
7. Put the six Settings content cards below the title bar into a two-column desktop grid while keeping mobile single-column flow.
8. Keep the WebDAV card internals single-column inside the half-width desktop card so URL and labels remain readable.
9. Reorder the six Settings cards into three explicit desktop rows: Account/Security, WebDAV/API Tokens, OCR Status/Storage, with each row stretching left and right cards to the same height.

## Acceptance

- Settings no longer renders the Appearance card or theme option buttons.
- WebDAV Access still shows `/dav` and never renders the current auth token.
- WebDAV instructions are grouped by connection details, credentials, setup order, and client notes.
- WebDAV card internals remain readable inside the half-width desktop card.
- Settings Center title bar stays full-width above the six-card desktop grid.
- The six Settings content cards use a two-column desktop layout and a single-column mobile layout.
- The desktop row order is Account/Security, WebDAV Access/API Tokens, OCR Status/Storage.
- Each desktop row stretches the left and right cards to equal height without making all rows the same height.
- Focused Settings regression tests pass.
