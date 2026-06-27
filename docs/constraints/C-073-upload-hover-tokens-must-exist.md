# C-073: Upload Hover Styles Must Use Existing Theme Tokens

Date: 2026-05-12

## Rule

Upload dialog hover styles must only reference CSS color tokens that are
defined in `frontend/src/styles/tokens.css`.

## Why

Browsers drop an entire CSS declaration when a referenced custom property is
missing. A hover gradient that includes one undefined token can silently render
as the previous background, making visual tests and screenshots the only place
the failure is obvious.

## Required Pattern

- Before adding upload dialog hover colors, grep `tokens.css` for every
  `--rgb-*` custom property used by the new declaration.
- Keep hover styles scoped by theme and state, e.g. purple ready upload footer
  selectors.
- Add a regression test that asserts the scoped selector and the key token
  references.
- Capture visual evidence for hover states when the change is visual.
