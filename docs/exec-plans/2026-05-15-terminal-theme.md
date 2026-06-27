# Terminal Theme Exec Plan

## Summary

Add `Terminal` as the fourth frontend theme without replacing or changing the existing dark, light, and purple themes.

## Assumptions

- `Terminal` is persisted as the literal theme value `terminal`.
- The DOM should expose `data-theme="terminal"` and keep the `.dark` class for dark-mode base behavior.
- The UI display name is `Terminal`.
- Visual motion is CSS-only and respects reduced-motion preferences.

## Risks

- Theme state may accept `terminal` but fail to apply the terminal DOM class.
- The toggle chain can skip the new fourth theme if the cycle remains ternary.
- A token-only implementation can miss entry points such as `ThemeSection`.
- Heavy visual effects could make the app slower, so the theme only uses tokenized gradients, scanlines, and small CSS animations.

## Dependencies

- `frontend/src/store/themeStore.ts`
- `frontend/src/components/common/ThemeToggle.tsx`
- `frontend/src/components/settings/ThemeSection.tsx`
- `frontend/src/styles/tokens.css`
- Vitest, token checks, hardcoding checks, ESLint, and production build.

## Steps

1. Add failing store, toggle, settings UI, and token tests for `terminal`.
2. Extend the theme type and DOM application logic.
3. Update ThemeToggle labels, icon, and cycle order.
4. Add Terminal to ThemeSection.
5. Add Terminal token block and lightweight CSS motion.
6. Run focused tests and full frontend verification.
