---
name: feature-development-with-constraints-and-exec-plan
description: Workflow command scaffold for feature-development-with-constraints-and-exec-plan in upload-download-util.
allowed_tools: ["Bash", "Read", "Write", "Grep", "Glob"]
---

# /feature-development-with-constraints-and-exec-plan

Use this workflow when working on **feature-development-with-constraints-and-exec-plan** in `upload-download-util`.

## Goal

Implements a new feature or significant enhancement, accompanied by documentation of constraints and execution plans, and updates to quality metrics.

## Common Files

- `docs/constraints/C-*-*.md`
- `docs/exec-plans/*.json`
- `docs/exec-plans/*.md`
- `docs/quality-score.md`
- `frontend/src/components/**/*.{tsx,ts}`
- `frontend/src/utils/*.{ts,tsx}`

## Suggested Sequence

1. Understand the current state and failure mode before editing.
2. Make the smallest coherent change that satisfies the workflow goal.
3. Run the most relevant verification for touched files.
4. Summarize what changed and what still needs review.

## Typical Commit Signals

- Create or update one or more docs/constraints/C-XXX-*.md files to specify new or changed constraints.
- Create or update docs/exec-plans/YYYY-MM-DD-*.json and .md files to outline the execution plan for the feature.
- Update docs/quality-score.md to reflect the impact on quality metrics.
- Implement or update feature logic in relevant frontend/src/components/* files.
- Update or add related utility functions in frontend/src/utils/*.

## Notes

- Treat this as a scaffold, not a hard-coded script.
- Update the command if the workflow evolves materially.