# Purple Nebula Bloom Theme Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the existing purple mode into a dreamier cosmic Nebula Bloom theme without changing layout.

**Architecture:** This is a token-first visual retune. `themeStore` already maps `purple` to `[data-theme="purple"]` and `.purple`; implementation stays inside CSS custom properties plus token regression tests, leaving React components and layout CSS untouched unless verification proves a rendering bug.

**Tech Stack:** React 19, Vite, Tailwind CSS utilities, CSS custom properties, Vitest.

---

## Assumptions

- The request targets only the existing `purple` theme mode.
- "Do not affect layout" means no DOM, route, prop, store, sizing, spacing, component hierarchy, or persistence changes.
- Color, gradient, border, shadow, opacity, and glow token values may change.
- Existing visual hooks already consume semantic tokens across nav, footer, file list, upload, dialogs, settings, and preview.

## Risks

- A brighter nebula can reduce text contrast if panel and button tokens are not kept dark enough.
- Overly strong rose/fuchsia fields can make operational file-management screens feel decorative rather than usable.
- `frontend/src/styles/tokens.css` is broad; a single token can affect multiple surfaces.
- Fluid sizing governance must remain untouched and passing.

## Dependencies

- `frontend/src/styles/tokens.css`
- `frontend/src/styles/purpleThemeTokens.test.ts`
- `frontend/src/store/themeStore.ts`
- `frontend/src/components/layout/PageLayout.tsx`
- `frontend/src/styles/nav.css`
- `frontend/src/components/files/list/FileListGlass.css`
- `frontend/src/components/files/upload/UploadDialog.css`

## Task 1: Purple Token Regression

**Files:**

- Create: `frontend/src/styles/purpleThemeTokens.test.ts`

- [ ] **Step 1: Write the failing test**

Create a Vitest file that reads the `[data-theme="purple"], .purple` rule from
`tokens.css` and asserts that the page, nav/footer, and glass surfaces use a
Nebula Bloom palette: radial gradients, fuchsia, rose, cyan/sky, purple, and
star-dust white accents.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cd frontend
npm run test -- src/styles/purpleThemeTokens.test.ts
```

Expected: FAIL because the current purple surface is mostly linear dark purple
and several purple tokens still use emerald/green accents.

## Task 2: Nebula Bloom Token Retune

**Files:**

- Modify: `frontend/src/styles/tokens.css`

- [ ] **Step 1: Implement minimal token changes**

Retune only the `[data-theme="purple"], .purple` block. Use layered radial
gradients for page/nav/footer surfaces, strengthen violet/rose/fuchsia nebula
tokens, keep cyan as a cool star-light accent, and reduce green/emerald accents
that conflict with the selected dreamier direction.

- [ ] **Step 2: Run focused test**

Run:

```bash
cd frontend
npm run test -- src/styles/purpleThemeTokens.test.ts
```

Expected: PASS.

## Task 3: Verification And Evidence

**Files:**

- Modify: `docs/quality-score.md`
- Create screenshot files under `docs/exec-plans/`

- [ ] **Step 1: Run governance and CI-equivalent checks**

Run:

```bash
cd frontend
npm run test -- src/styles/purpleThemeTokens.test.ts
npm run check:tokens:strict:layout
npm run check:fluid-sizing -- --scope=all
npm run lint
npm run test
npm run build
git diff --check
```

Expected: all commands pass.

- [ ] **Step 2: Capture screenshot evidence**

Start the frontend if needed and capture purple-mode screenshots:

- `docs/exec-plans/2026-05-05-purple-nebula-bloom-files-desktop.png`
- `docs/exec-plans/2026-05-05-purple-nebula-bloom-files-mobile.png`
- `docs/exec-plans/2026-05-05-purple-nebula-bloom-settings-desktop.png`
- `docs/exec-plans/2026-05-05-purple-nebula-bloom-settings-mobile.png`

- [ ] **Step 3: Update quality score**

Add a `2026-05-05` row to `docs/quality-score.md` summarizing the purple Nebula
Bloom score and verification evidence.

## Verification Commands

- `npm run test -- src/styles/purpleThemeTokens.test.ts`
- `npm run check:tokens:strict:layout`
- `npm run check:fluid-sizing -- --scope=all`
- `npm run lint`
- `npm run test`
- `npm run build`
- `git diff --check`

