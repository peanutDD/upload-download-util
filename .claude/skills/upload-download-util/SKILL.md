```markdown
# upload-download-util Development Patterns

> Auto-generated skill from repository analysis

## Overview

This skill provides a comprehensive guide to the development patterns, coding conventions, and common workflows in the `upload-download-util` Rust repository. The project is focused on backend and CLI utilities for file upload/download operations, with a modular architecture and AI-powered automation for code review and workflow integration. The repository emphasizes maintainability, observability, and clear separation of concerns, making it suitable for scalable and robust backend systems.

## Coding Conventions

- **File Naming:**  
  Use `camelCase` for file names.  
  _Example:_  
  ```
  uploadHandler.rs
  fileListManager.rs
  ```

- **Import Style:**  
  Use **relative imports** within modules.  
  _Example:_  
  ```rust
  mod config;
  use crate::config::AppConfig;
  ```

- **Export Style:**  
  Mixed. Some modules use explicit `pub` exports; others re-export via `mod.rs`.  
  _Example:_  
  ```rust
  // In src/config/mod.rs
  pub mod database;
  pub use database::DatabaseConfig;
  ```

- **Commit Messages:**  
  Follow [Conventional Commits](https://www.conventionalcommits.org/) with these prefixes:  
  - `feat`: New feature
  - `fix`: Bug fix
  - `refactor`: Code refactoring
  - `docs`: Documentation changes
  - `chore`: Maintenance tasks
  - `ci`: Continuous integration

  _Example:_  
  ```
  feat: add tracing initialization to backend
  fix: correct file list pagination bug
  ```

## Workflows

### Backend Config Refactor
**Trigger:** When restructuring backend configuration for maintainability or new features  
**Command:** `/refactor-config`

1. Split or reorganize `backend/src/config/*.rs` files (create new modules, move fields).
2. Update `backend/src/config/mod.rs` to include new modules.
3. Update all backend code referencing config fields to new paths.
4. Update `backend/src/main.rs` and related entry points.
5. Update environment variable examples in `backend/.env.example`.
6. Update documentation if necessary.

_Code Example:_
```rust
// Before
use crate::config::DatabaseConfig;

// After splitting config
use crate::config::database::DatabaseConfig;
```

---

### Add or Enhance Observability
**Trigger:** When adding or improving distributed tracing/observability in the backend  
**Command:** `/add-tracing`

1. Add or update tracing modules (`backend/src/tracing.rs`).
2. Update `backend/Cargo.toml` to add/upgrade dependencies (e.g., `opentelemetry`).
3. Update `backend/.env.example` and documentation for new config.
4. Modify `backend/src/main.rs` and worker entry points for tracing initialization.
5. Update or add `docker-compose.yml` for tracing services (e.g., Jaeger).
6. Document changes in `docs/CHANGELOG.md`.

_Code Example:_
```rust
// backend/src/tracing.rs
pub fn init_tracing() {
    // Setup OpenTelemetry tracing
}
```

---

### Codex CLI Skill or Pipeline Extension
**Trigger:** When adding new skills, extending the pipeline, or modularizing `codex-cli` for AI-powered PR automation  
**Command:** `/add-codex-skill`

1. Add or refactor `scripts/codex-cli/src/skills.rs` and related modules.
2. Update or split `scripts/codex-cli/src/main.rs` into modular files (`lib.rs`, `pipeline.rs`, etc.).
3. Update or add `scripts/codex-cli/src/bin/codex.rs` as binary entrypoint.
4. Update `scripts/codex-cli/Cargo.toml` for new binaries or dependencies.
5. Update or add documentation in `docs/design-docs/*` and `docs/references/*`.
6. Update `.github/workflows/ai-auto-fix.yml` for workflow integration.
7. Document changes in `docs/CHANGELOG.md`.

_Code Example:_
```rust
// scripts/codex-cli/src/skills.rs
pub fn new_skill() {
    // Implementation of new CLI skill
}
```

---

### Backend Entity/DTO Separation
**Trigger:** When refactoring backend models for clearer separation between database and API layers  
**Command:** `/separate-entity-dto`

1. Create `backend/src/entities/*.rs` for DB models.
2. Create `backend/src/types/*.rs` for API DTOs.
3. Update `backend/src/models/*.rs` to re-export or adapt to new modules.
4. Update handlers, services, and other backend code to use new types.
5. Update or add documentation to explain new architecture.

_Code Example:_
```rust
// backend/src/entities/user.rs
pub struct UserEntity { /* DB fields */ }

// backend/src/types/user.rs
pub struct UserDTO { /* API fields */ }
```

---

### Update or Add AI Auto-Fix Workflow
**Trigger:** When introducing or improving automated code review and fixing via AI  
**Command:** `/add-ai-auto-fix`

1. Add or update `.github/workflows/ai-auto-fix.yml`.
2. Add or update `scripts/codex-cli/Cargo.toml` and src files.
3. Add or update `docs/CHANGELOG.md` and `docs/design-docs/auto-review-flow.md`.
4. Update `docs/references/workflow-integration.md`.
5. Add or update constraint/usage documentation.

---

### Frontend File List Feature or Bugfix
**Trigger:** When adding, optimizing, or fixing file listing, grid virtualization, or deletion UI in the frontend  
**Command:** `/update-file-list`

1. Update `frontend/src/components/files/grid/*.tsx` or `useFileList.ts`.
2. Update `frontend/src/hooks/files/useFileMutations.ts`.
3. Add or update `frontend/src/utils/*.ts` for measurement or helper logic.
4. Update related tests if needed.

---

## Testing Patterns

- **Test File Naming:**  
  Test files use the pattern `*.test.*` (e.g., `uploadHandler.test.rs`).

- **Framework:**  
  No specific test framework detected; likely using Rust's built-in test framework.

- **Test Example:**  
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_upload_success() {
          // Test logic here
      }
  }
  ```

## Commands

| Command             | Purpose                                                        |
|---------------------|----------------------------------------------------------------|
| /refactor-config    | Refactor or restructure backend configuration modules          |
| /add-tracing        | Add or enhance distributed tracing/observability               |
| /add-codex-skill    | Add or refactor CLI skills or pipeline modules                |
| /separate-entity-dto| Separate backend DB entities from API DTOs                    |
| /add-ai-auto-fix    | Add or enhance AI-powered PR auto-fix workflow                |
| /update-file-list   | Implement or fix frontend file list/grid features              |
```
