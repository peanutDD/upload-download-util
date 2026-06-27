# Settings Redesigned Layout Execution Plan

Date: 2026-05-15

## Goal

Replace the rigid two-column equal-height Settings layout with a more polished,
task-oriented layout that keeps inputs, explanatory text, and confirmation
actions visually calm and easy to scan.

## Assumptions

- Visual quality and clarity now take priority over the previous strict
  two-column, three-row equal-height requirement.
- `Settings Center` remains the full-width title and summary area.
- Appearance stays removed from the Settings page.
- Form behavior, API payloads, auth state, token creation, and WebDAV details
  remain unchanged.

## Risks

- API Tokens and WebDAV Access contain more content than the other cards, so
  forcing them into paired columns creates visual imbalance.
- Moving cards into wider sections can lengthen the page, but improves reading
  and operation rhythm.
- Styling form actions must not change submit buttons, labels, or registered
  input names.

## Dependencies

- `frontend/src/pages/Settings.tsx`
- `frontend/src/components/settings/WebDavAccessSection.tsx`
- `frontend/src/components/settings/UserInfoSection.tsx`
- `frontend/src/components/settings/PasswordChangeSection.tsx`
- `frontend/src/components/settings/ApiTokenSection.tsx`
- `frontend/src/components/settings/SettingsPageRegression.test.tsx`
- Existing Settings card and semantic design tokens.

## Steps

1. Add failing Settings regression coverage for the new focused layout groups.
2. Replace the strict equal-height rows with identity, WebDAV focus, token
   workspace, and status groups.
3. Redesign WebDAV Access as a full-width setting area with one primary
   connection panel and three supporting guidance panels.
4. Align form action buttons in calm bottom action areas for Account, Security,
   and API Tokens.
5. Keep business behavior unchanged and verify token values are not exposed.
6. Capture desktop and mobile screenshots to confirm the page is visually
   balanced without clipped controls or awkward input/button alignment.
7. Update quality score after tests, lint, build, and visual verification pass.

## Acceptance

- Settings Center stays outside the content groups.
- Account and Security remain near each other without forced equal-height
  stretching.
- WebDAV Access uses a balanced full-width presentation and keeps `/dav`
  visible.
- API Tokens has enough horizontal space for its create form, toggles, root
  input, and existing-token list.
- OCR Status and Storage remain paired as status-oriented sections.
- Account, Security, and API Tokens confirmation buttons align consistently.
- Focused Settings regression tests, lint, build, and screenshot checks pass.
