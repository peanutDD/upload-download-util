---
name: backend-config-refactor
description: Workflow command scaffold for backend-config-refactor in upload-download-util.
allowed_tools: ["Bash", "Read", "Write", "Grep", "Glob"]
---

# /backend-config-refactor

Use this workflow when working on **backend-config-refactor** in `upload-download-util`.

## Goal

Refactor or restructure backend configuration modules, often splitting or reorganizing config files and updating all references.

## Common Files

- `backend/src/config/*.rs`
- `backend/src/config/mod.rs`
- `backend/src/main.rs`
- `backend/.env.example`

## Suggested Sequence

1. Understand the current state and failure mode before editing.
2. Make the smallest coherent change that satisfies the workflow goal.
3. Run the most relevant verification for touched files.
4. Summarize what changed and what still needs review.

## Typical Commit Signals

- Split or reorganize backend/src/config/*.rs files (e.g., create new modules, move fields)
- Update backend/src/config/mod.rs to include new modules
- Update all backend code referencing config fields to new paths
- Update backend/src/main.rs and related entry points
- Update environment variable examples (backend/.env.example)

## Notes

- Treat this as a scaffold, not a hard-coded script.
- Update the command if the workflow evolves materially.