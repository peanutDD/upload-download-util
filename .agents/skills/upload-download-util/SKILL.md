```markdown
# upload-download-util Development Patterns

> Auto-generated skill from repository analysis

## Overview
This skill teaches you the core development patterns and conventions used in the `upload-download-util` Rust repository. You'll learn how to structure files, organize imports and exports, write and discover tests, and follow the project's coding style. The guide also provides step-by-step workflows and suggested commands for common development tasks.

## Coding Conventions

### File Naming
- Use **kebab-case** for all file names.
  - Example: `file-handler.rs`, `upload-manager.rs`

### Import Style
- Use **relative imports** to reference modules within the project.
  - Example:
    ```rust
    mod utils;
    use crate::utils::file_ops;
    ```

### Export Style
- Use **named exports** to expose functions, structs, or modules.
  - Example:
    ```rust
    pub fn upload_file(...) { ... }
    pub struct DownloadConfig { ... }
    ```

### Commit Messages
- Freeform, no enforced prefixes.
- Average length: ~44 characters.
  - Example:  
    ```
    Add support for multipart uploads
    ```

## Workflows

### Adding a New Utility Function
**Trigger:** When you need to add a new helper function.
**Command:** `/add-util-function`

1. Create a new file in `src/` using kebab-case (e.g., `my-helper.rs`).
2. Implement your function and use `pub` to export it.
3. Add a relative import in the parent module.
4. Write a corresponding test in a file matching `*.test.*`.
5. Commit your changes with a clear, concise message.

### Running Tests
**Trigger:** When you want to verify code correctness.
**Command:** `/run-tests`

1. Locate test files matching the `*.test.*` pattern.
2. Use Rust's built-in test runner:
    ```sh
    cargo test
    ```
3. Review the output for passing/failing tests.

### Refactoring Imports
**Trigger:** When reorganizing or cleaning up module imports.
**Command:** `/refactor-imports`

1. Ensure all imports are relative.
2. Update import paths to use `crate::` or `super::` as appropriate.
3. Remove unused imports.

## Testing Patterns

- Test files follow the `*.test.*` pattern (e.g., `file-handler.test.rs`).
- The testing framework is not explicitly specified; likely uses Rust's built-in testing.
- Example test structure:
    ```rust
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_upload_file() {
            // test logic here
        }
    }
    ```

## Commands
| Command            | Purpose                                   |
|--------------------|-------------------------------------------|
| /add-util-function | Add a new utility function/module         |
| /run-tests         | Run all tests in the repository           |
| /refactor-imports  | Refactor and clean up import statements   |
```
