# Root Directory Inventory

> Date: 2026-05-07
> Scope: repository root of `upload-download-util`
> Purpose: record what each root-level file or directory is for, and whether it can be deleted or moved during cleanup.

## Decision Legend

| Decision | Meaning |
| --- | --- |
| Keep | Must remain in the repository root or is a normal root-level project entry. |
| Move | Content is useful, but should live under `docs/`, `scripts/`, or another focused directory. |
| Delete | Safe or recommended to remove from the repository root. |
| Confirm | Do not remove until the owning local tool or workflow is confirmed. |

## Root Items

| Item | Purpose | Decision | Notes |
| --- | --- | --- | --- |
| `.git/` | Git repository metadata. | Keep | Deleting it would detach the directory from Git history. |
| `AGENTS.md` | Project-level Agent rules and execution constraints. | Keep | Must stay at root because Agent tools load it from the repository root. |
| `README.md` | Main project introduction, feature list, quick start, and documentation links. | Keep | Standard root entry document. |
| `CONTRIBUTING.md` | Contribution and PR process guidance. | Keep | Referenced by `README.md`. |
| `.gitignore` | Git ignore rules for local artifacts, build outputs, uploads, logs, and runner files. | Keep | Required for clean version control. |
| `.dockerignore` | Docker build context ignore rules. | Keep | Prevents local/generated files from entering Docker build context. |
| `.vercelignore` | Vercel deployment ignore rules. | Keep | Keep if Vercel deployment remains supported. |
| `.github/` | GitHub Actions workflows, workflow scripts, and PR template. | Keep | GitHub requires these paths. |
| `backend/` | Rust/Axum backend source, migrations, backend docs, and backend scripts. | Keep | Core project source. |
| `frontend/` | React/TypeScript frontend source and frontend build configuration. | Keep | Core project source. |
| `docs/` | Project documentation, constraints, design docs, execution plans, references, and quality score. | Keep | Project memory and Agent-readable documentation. |
| `scripts/` | Project scripts and the `codex-cli` utility workspace. | Keep | Active tooling lives here. |
| `deploy/` | Deployment configuration, including nginx config. | Keep | Project deployment support. |
| `docker-compose.yml` | Local infrastructure startup for Postgres, Redis, and tracing dependencies. | Keep | Referenced by project quick-start flow. |
| `rust-toolchain.toml` | Rust toolchain pin/selection for the repository. | Keep | Root-level convention used by Rust tooling. |
| `vercel.json` | Vercel SPA rewrite/deployment config. | Keep | Required if deploying frontend through Vercel from repo root. |
| `CLAUDE.md` | Claude/Agent-specific rule entry. | Confirm | Could be auto-loaded by Claude Code or related tools; only move/delete after confirming it is unused. |
| `UNIVERSAL-AGENT.md` | General Agent harness/rules document. | Confirm | Can likely move to `docs/references/`, but keep at root if external workflows pass it by root path. |
| `.cursor/` | Cursor rules and local skills. | Confirm | Currently ignored by `.gitignore`; keep locally if Cursor workflows depend on it. |
| `.agents/` | Local Agent skills, including a YouTube skill. | Confirm | Looks like personal/tool state; remove only if those local skills are no longer needed. |
| `skills-lock.json` | Lock file matching `.agents` skills. | Confirm | Delete together with `.agents/` if the local skills are removed. |
| `.trae/` | Trae documents and skills. | Confirm | Some documents may be worth archiving into `docs/`; do not delete if Trae still scans `.trae/skills`. |
| `.agent-hooks.md` | Agent hooks notes or local hook documentation. | Move | Recommended destination: `docs/references/agent-hooks.md`. |
| `.superpowers/` | Local Superpowers session/state data. | Delete | Ignored by Git; deleting may lose local session history only. |
| `.DS_Store` | macOS Finder metadata. | Delete | Generated OS artifact; already ignored. |
| `node_modules/` | Root-level npm dependency cache. | Delete | Rebuildable via package manager; root npm project appears to be accidental. |
| `actions-runner/` | GitHub self-hosted runner installation and working directory. | Confirm | Very large local runtime directory; delete only if the local runner is not in use. |
| `.rustup/` | Repository-local Rustup state. | Delete | Project already has `rust-toolchain.toml`; confirm no script sets `RUSTUP_HOME` to this path. |
| `.superset/` | Local Superset/tool config. | Delete | No active project reference was found. |
| `.vercel/` | Vercel CLI local project link. | Delete | Deleting is safe for source cleanup; local Vercel CLI may need `vercel link` again. |
| `codex-auto-fix.yml` | Historical root-level workflow copy. | Delete | Canonical workflow lives in `.github/workflows/codex-auto-fix.yml`; root copy should not be used. |
| `gemini_review.txt` | Gemini review example or temporary review text. | Move | Recommended destination: `docs/references/` or a test fixture directory if used by tests. |
| `ISSUES.md` | Historical issue list or backlog notes. | Move | Recommended destination: `docs/ISSUES.md` or `docs/references/ISSUES.md`. |
| `START_SYSTEM.sh` | Manual system startup script. | Move | Recommended destination: `scripts/START_SYSTEM.sh`; update any references after moving. |
| `create_database.sql` | Database initialization helper script. | Move | Recommended destination: `scripts/db/create_database.sql`; update `scripts/dev.sh`, which currently references the root path. |
| `package.json` | Root-level npm config containing only an `audit` dependency. | Delete | Looks like accidental root install; frontend has its own `frontend/package.json`. |
| `package-lock.json` | Lock file for the root-level npm config. | Delete | Delete together with root `package.json`. |
| `bun.lock` | Root-level Bun lock file. | Delete | No root Bun project config was found. |

## Recommended Cleanup Order

1. Delete low-risk generated artifacts: `.DS_Store`, `node_modules/`, `.superpowers/`.
2. Remove suspicious local/tool state after quick confirmation: `.rustup/`, `.superset/`, `.vercel/`, root `package.json`, root `package-lock.json`, `bun.lock`.
3. Move useful documents and scripts into focused locations: `.agent-hooks.md`, `gemini_review.txt`, `ISSUES.md`, `START_SYSTEM.sh`, `create_database.sql`.
4. Decide separately on tool-specific folders: `.cursor/`, `.agents/`, `.trae/`, `actions-runner/`.

## High-Risk Item

`actions-runner/` is the biggest cleanup candidate by size, but it may contain a configured self-hosted GitHub Actions runner, logs, credentials, and active working directories. Do not delete it until the local runner service status is checked and the owner confirms it is no longer needed.

