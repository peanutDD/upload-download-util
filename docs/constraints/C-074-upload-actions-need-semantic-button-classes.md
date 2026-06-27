# C-074: Upload Actions Need Semantic Button Classes

Date: 2026-05-12

## Rule

Upload dialog actions must not rely only on the generic primary/secondary
button classes when their role differs. `Cancel`, disabled `Attach files`,
`Select files`, and URL `Upload` each need a semantic class before applying
role-specific colors.

## Why

The upload dialog has several actions with different intent: close, inactive
footer action, local file selection, and remote URL upload. Sharing one primary
class makes visual changes bleed across roles and makes empty-state styling hard
to reason about.

## Required Pattern

- Keep a generic button foundation for spacing and base chrome.
- Add semantic classes such as `uploadDialogSelectFilesBtn` or
  `uploadDialogUrlUploadBtn` for role-specific palette decisions.
- Scope empty footer action styling with
  `data-ready-to-upload="false"` when it only applies before files are queued.
- Cover semantic classes and palette tokens with a regression test.
