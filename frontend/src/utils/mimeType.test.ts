import { describe, expect, it } from 'vitest';
import { getPreviewKind } from './mimeType';

describe('mimeType preview detection', () => {
  it('falls back to filename extension for legacy octet-stream metadata', () => {
    const kind = getPreviewKind('application/octet-stream', 'finder-image.png');

    expect(kind.supported).toBe(true);
    expect(kind.isImage).toBe(true);
  });
});
