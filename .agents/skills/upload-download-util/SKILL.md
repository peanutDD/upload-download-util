```markdown
# upload-download-util Development Patterns

> Auto-generated skill from repository analysis

## Overview
This skill teaches best practices and conventions for contributing to the `upload-download-util` TypeScript utility library. You'll learn the project's file organization, code style, import/export patterns, and how to write and run tests using Vitest. This guide ensures consistency and efficiency when developing or maintaining the codebase.

## Coding Conventions

### File Naming
- Use **PascalCase** for all file names.
  - Example: `FileUploader.ts`, `DownloadManager.ts`

### Import Style
- Always use **relative imports**.
  - Example:
    ```typescript
    import FileUploader from './FileUploader';
    ```

### Export Style
- Use **default exports** for modules.
  - Example:
    ```typescript
    // FileUploader.ts
    const FileUploader = () => { /* ... */ };
    export default FileUploader;
    ```

### Commit Messages
- Freeform style, no enforced prefixes.
- Average length: ~45 characters.
  - Example:  
    ```
    Add support for multiple file uploads
    ```

## Workflows

### Adding a New Utility
**Trigger:** When you need to add a new upload/download utility function or class.  
**Command:** `/add-utility`

1. Create a new file using PascalCase (e.g., `MyUtility.ts`).
2. Implement your utility using TypeScript.
3. Use relative imports for dependencies.
4. Export the utility as the default export.
5. Write a corresponding test file (see Testing Patterns).
6. Commit your changes with a clear, concise message.

### Running Tests
**Trigger:** When you want to verify code correctness or before submitting a pull request.  
**Command:** `/run-tests`

1. Ensure all test files are named with the `.test.tsx` suffix.
2. Run the test suite using Vitest:
    ```bash
    npx vitest
    ```
3. Review test results and fix any failing tests.

### Refactoring Existing Code
**Trigger:** When improving or reorganizing existing utilities.  
**Command:** `/refactor`

1. Update the relevant `.ts` files, maintaining PascalCase naming.
2. Use relative imports and default exports.
3. Update or add tests as needed.
4. Commit changes with a descriptive message.

## Testing Patterns

- All tests use the **Vitest** framework.
- Test files are named with the `.test.tsx` suffix and placed alongside the code they test.
  - Example: `FileUploader.test.tsx`
- Typical test structure:
    ```typescript
    import { describe, it, expect } from 'vitest';
    import FileUploader from './FileUploader';

    describe('FileUploader', () => {
      it('uploads a file successfully', () => {
        // test implementation
        expect(/* ... */).toBe(/* ... */);
      });
    });
    ```

## Commands
| Command        | Purpose                                               |
|----------------|-------------------------------------------------------|
| /add-utility   | Scaffold and add a new upload/download utility        |
| /run-tests     | Run the full Vitest test suite                        |
| /refactor      | Refactor existing code while following conventions    |
```
