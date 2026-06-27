# C-072: Unselected Card Checkboxes Need Their Own Contrast Treatment

Date: 2026-05-12

## Rule

Unselected file/folder card checkboxes must have a distinct outer surface,
outer border, shadow, and inner ring treatment. They must not rely only on a
small transparent inner border over the thumbnail background.

## Why

File and folder thumbnails can be dark, bright, or visually busy. A tiny inner
ring on a nearly transparent surface gets lost against the card background,
especially on mobile where the checkbox is always visible and must be
discoverable without hover.

## Required Pattern

- Use a semantic unselected checkbox class for the outer circle.
- Use a separate semantic class for the inner unselected ring.
- Drive colors and shadows through theme tokens.
- Keep selected checkbox styling separate from unselected styling.
- Cover this with a regression test that asserts unselected cards render the
  high-contrast classes.
