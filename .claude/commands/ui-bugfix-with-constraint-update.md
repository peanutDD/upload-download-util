---
name: ui-bugfix-with-constraint-update
description: Workflow command scaffold for ui-bugfix-with-constraint-update in upload-download-util.
allowed_tools: ["Bash", "Read", "Write", "Grep", "Glob"]
---

# /ui-bugfix-with-constraint-update

Use this workflow when working on **ui-bugfix-with-constraint-update** in `upload-download-util`.

## Goal

Fixes a UI bug and updates or adds relevant constraint documentation to ensure the fix is tracked and requirements are clear.

## Common Files

- `docs/constraints/C-*-*.md`
- `frontend/src/components/**/*.{tsx,ts}`
- `frontend/src/components/**/*.{test.tsx,test.ts}`
- `frontend/src/components/**/*.css`

## Suggested Sequence

1. Understand the current state and failure mode before editing.
2. Make the smallest coherent change that satisfies the workflow goal.
3. Run the most relevant verification for touched files.
4. Summarize what changed and what still needs review.

## Typical Commit Signals

- Update or add a docs/constraints/C-XXX-*.md file to document the constraint or bugfix requirement.
- Modify relevant frontend/src/components/* files to fix the bug.
- Add or update test files in frontend/src/components/*/*.test.tsx.
- Update related stylesheets if necessary.

## Notes

- Treat this as a scaffold, not a hard-coded script.
- Update the command if the workflow evolves materially.