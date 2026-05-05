# Purple Nebula Bloom Theme

Date: 2026-05-05

## Intent

Retune the existing `purple` theme into a dreamier Nebula Bloom visual style:
deep violet space, brighter violet and rose nebula clouds, cool cyan star
light, and glass surfaces that feel luminous without changing layout.

## User Approval

The user selected direction C, "Nebula Bloom", from the visual companion and
confirmed this design before implementation.

## Scope

Included:

- Purple theme CSS token changes.
- Purple-mode regression tests for the new cosmic palette.
- Screenshot evidence for `/files` and `/settings` in desktop and mobile
  widths.
- Quality score documentation after verification.

Excluded:

- DOM structure, route, prop, store, persistence, spacing, sizing, or component
  hierarchy changes.
- New theme modes.
- Animation-heavy overlays or layout-affecting decorative elements.

## Design

The purple page surface should become a layered nebula field rather than a
plain dark-purple gradient. The base remains dark enough for long file-management
sessions, while radial violet, fuchsia, rose, and cyan light fields create the
dreamier star-cloud tone requested by the user.

Navigation and footer chrome should inherit the same cosmic palette: dark violet
glass, soft rose/fuchsia edges, cyan star highlights, and restrained white
star-dust accents. Buttons and toolbar surfaces should keep their current
dimensions and component structure while receiving more luminous glass tokens.

Dialogs, upload surfaces, file-list controls, and preview chrome should continue
to use existing semantic variables. The implementation should retune only token
values so the current component CSS remains the single layout authority.

## Testing

- Add a purple token regression test before implementation.
- Verify the test fails against the current purple palette.
- Update token values until the purple token test passes.
- Run layout/token governance, lint, full tests, build, and screenshot capture.

