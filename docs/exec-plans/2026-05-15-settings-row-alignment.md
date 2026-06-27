# Settings Row Alignment Execution Plan

Date: 2026-05-15

## Goal

Align the first-row `Save` and `Change password` actions, and align the bottom
edges of the third-row `OCR Status` and `Storage` cards on desktop.

## Assumptions

- The first row means the `Account` and `Security` cards.
- The third row means the final status pair: `OCR Status` and `Storage`.
- This is a layout-only change; form behavior and API calls stay unchanged.

## Risks

- Stretching unequal content can introduce extra internal whitespace.
- The extra whitespace is acceptable when it creates cleaner action and card
  baselines.

## Steps

1. Add Settings regression coverage for first-row action alignment and status
   row stretching.
2. Stretch the `Account` and `Security` desktop columns and make their card
   content fill available height.
3. Move the `Account` and `Security` action rows to the bottom with `mt-auto`.
4. Stretch the `OCR Status` and `Storage` desktop columns so their bottom edges
   align.
5. Verify with focused tests, lint, build, and a desktop screenshot/DOM probe.

## Acceptance

- `Save` and `Change password` align horizontally on desktop.
- `OCR Status` and `Storage` have matching top and bottom edges on desktop.
- Focused Settings regression tests pass.
- Lint, build, and visual probe pass.
