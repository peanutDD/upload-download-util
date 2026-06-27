# Portfolio Theme Exec Plan

## Summary

Add `Portfolio` as the fifth frontend theme, inspired by the CodePen reference's neon developer portfolio style: deep slate blackglass, violet/cyan gradients, gold accents, code-grid texture, and lightweight CSS glow motion.

## Assumptions

- `Portfolio` is persisted as the literal theme value `portfolio`.
- The DOM should expose `data-theme="portfolio"` and keep the `.dark` class for dark-mode base behavior.
- The UI display name is `Portfolio`.
- The theme is a new fifth option and does not replace Terminal.
- Motion is CSS-only, lightweight, and respects reduced-motion preferences.

## Risks

- The toggle cycle can skip Portfolio if only the picker is updated.
- Portfolio can be persisted without applying the `.portfolio` DOM class.
- A token-only change can miss ThemeToggle or ThemeSection.
- The new palette can drift too close to Terminal unless it keeps violet/cyan/gold portfolio cues.

## Dependencies

- `frontend/src/store/themeStore.ts`
- `frontend/src/components/common/ThemeToggle.tsx`
- `frontend/src/components/settings/ThemeSection.tsx`
- `frontend/src/styles/tokens.css`
- Vitest, token checks, hardcoding checks, ESLint, production build, and Chrome visual smoke.

## Steps

1. Add failing store, toggle, settings UI, and token tests for `portfolio`.
2. Extend the theme type, DOM classes, and toggle order.
3. Add Portfolio to ThemeToggle and ThemeSection.
4. Add Portfolio token block and lightweight CSS motion.
5. Run focused tests and full frontend verification.
6. Capture desktop and mobile Chrome evidence screenshots.
