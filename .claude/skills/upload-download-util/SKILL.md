```markdown
# upload-download-util Development Patterns

> Auto-generated skill from repository analysis

## Overview
This skill teaches you how to contribute to the `upload-download-util` TypeScript codebase, which provides utilities for file upload and download functionality. You'll learn the project's coding conventions, how to implement features or bugfixes with formal documentation and quality tracking, and how to write and organize tests using Vitest.

## Coding Conventions

- **Language:** TypeScript
- **Framework:** None detected (vanilla TypeScript)
- **File Naming:** PascalCase for files (e.g., `UploadManager.ts`)
- **Import Style:** Relative imports  
  ```ts
  import UploadManager from './UploadManager';
  ```
- **Export Style:** Default exports  
  ```ts
  export default UploadManager;
  ```
- **Commit Patterns:**  
  - Freeform messages, sometimes prefixed with `feat`
  - Example: `feat: add drag-and-drop upload support`
- **Documentation:**  
  - Constraints and plans are documented in `docs/constraints/` and `docs/exec-plans/`
  - Quality metrics tracked in `docs/quality-score.md`

## Workflows

### Feature Development with Constraints and Exec Plan
**Trigger:** When adding a significant new feature or enhancement, with formal documentation and quality tracking  
**Command:** `/new-feature-with-constraints`

1. **Document Constraints:**  
   Create or update one or more files in `docs/constraints/` (e.g., `docs/constraints/C-123-upload-limits.md`) to specify new or changed requirements.
2. **Plan Execution:**  
   Create or update execution plan files in `docs/exec-plans/` (e.g., `docs/exec-plans/2024-06-10-upload-refactor.md` and `2024-06-10-upload-refactor.json`) outlining the steps and considerations for the feature.
3. **Update Quality Metrics:**  
   Edit `docs/quality-score.md` to reflect the expected impact on quality.
4. **Implement Feature:**  
   Add or update logic in `frontend/src/components/` (e.g., `frontend/src/components/UploadManager.tsx`).
5. **Update Utilities:**  
   Modify or add utility functions in `frontend/src/utils/` as needed.
6. **Write/Update Tests:**  
   Write or update tests in:
   - `frontend/src/components/ComponentName/ComponentName.test.tsx`
   - `frontend/src/utils/UtilityName.test.ts`

**Example:**
```ts
// frontend/src/components/UploadManager.tsx
import React from 'react';
import uploadFile from '../utils/uploadFile';

const UploadManager = () => {
  // component logic
};

export default UploadManager;
```

### UI Bugfix with Constraint Update
**Trigger:** When fixing a UI bug that relates to a documented constraint  
**Command:** `/fix-ui-bug-with-constraint`

1. **Document/Update Constraint:**  
   Update or add a relevant file in `docs/constraints/` (e.g., `docs/constraints/C-234-fix-progress-bar.md`) to clarify the bugfix requirement.
2. **Fix the Bug:**  
   Modify the relevant component(s) in `frontend/src/components/` (e.g., `frontend/src/components/ProgressBar.tsx`).
3. **Update/Add Tests:**  
   Add or update tests in `frontend/src/components/ComponentName/ComponentName.test.tsx`.
4. **Update Styles (if needed):**  
   Modify CSS files in `frontend/src/components/ComponentName/ComponentName.css` if the bugfix requires style changes.

**Example:**
```ts
// frontend/src/components/ProgressBar.tsx
export default function ProgressBar({ progress }: { progress: number }) {
  return <div style={{ width: `${progress}%` }} />;
}
```

## Testing Patterns

- **Framework:** [Vitest](https://vitest.dev/)
- **Test File Naming:**  
  - For components: `ComponentName.test.tsx`  
  - For utilities: `UtilityName.test.ts`
- **Location:**  
  - Component tests: `frontend/src/components/ComponentName/ComponentName.test.tsx`
  - Utility tests: `frontend/src/utils/UtilityName.test.ts`
- **Example:**
  ```ts
  // frontend/src/utils/uploadFile.test.ts
  import uploadFile from './uploadFile';
  import { describe, it, expect } from 'vitest';

  describe('uploadFile', () => {
    it('uploads a file successfully', async () => {
      // test logic
    });
  });
  ```

## Commands

| Command                        | Purpose                                                      |
|--------------------------------|--------------------------------------------------------------|
| /new-feature-with-constraints  | Start a new feature with constraints and execution plan docs  |
| /fix-ui-bug-with-constraint    | Fix a UI bug and update related constraint documentation      |
```
