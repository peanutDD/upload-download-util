```markdown
# upload-download-util Development Patterns

> Auto-generated skill from repository analysis

## Overview
This skill covers the development patterns and conventions used in the `upload-download-util` Rust repository. It documents file naming, import/export styles, commit patterns, and testing strategies. While no explicit workflows were detected, this guide provides best practices and suggested commands for common development tasks.

## Coding Conventions

### File Naming
- Use **camelCase** for file names.
  - Example: `fileUploader.rs`, `dataParser.rs`

### Import Style
- Use **relative imports** within the codebase.
  - Example:
    ```rust
    mod fileUploader;
    use crate::fileUploader::upload_file;
    ```

### Export Style
- Mixed export styles are used (both explicit and implicit).
  - Example:
    ```rust
    pub fn upload_file(...) { ... }
    ```
    or
    ```rust
    pub mod fileUploader;
    ```

### Commit Patterns
- Commit messages are **freeform** (no enforced prefixes).
- Average commit message length: **51 characters**.
  - Example:  
    ```
    Add support for multipart file uploads
    ```

## Workflows

### Testing the Codebase
**Trigger:** When you want to run the test suite to verify code changes.
**Command:** `/test`

1. Ensure you have all dependencies installed.
2. Run the test suite using the appropriate command (see below).
3. Review test results and address any failures.

### Adding a New Utility Function
**Trigger:** When you need to add a new upload/download utility.
**Command:** `/add-util`

1. Create a new file using camelCase naming (e.g., `newUtility.rs`).
2. Implement the utility function.
3. Use relative imports to include it where needed.
4. Export the function using `pub`.
5. Write corresponding tests in a `.test.tsx` file (see Testing Patterns).

## Testing Patterns

- **Framework:** vitest (note: this is a JavaScript/TypeScript testing framework, which may indicate some JS/TS interop or frontend code).
- **Test file pattern:** `*.test.tsx`
- **Typical test file example:**
  ```typescript
  // fileUploader.test.tsx
  import { uploadFile } from './fileUploader';

  test('uploads file successfully', () => {
    // ...test implementation
  });
  ```

## Commands
| Command     | Purpose                                      |
|-------------|----------------------------------------------|
| /test       | Run the test suite                           |
| /add-util   | Add a new utility function/module            |
```