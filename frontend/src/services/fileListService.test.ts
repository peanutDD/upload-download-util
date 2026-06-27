import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AxiosHeaders } from 'axios';
import { limitedApi } from './api';
import { fileListService } from './fileListService';

vi.mock('./api', () => ({
  default: {
    delete: vi.fn(),
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
  },
  limitedApi: {
    get: vi.fn(),
  },
}));

const limitedGet = vi.mocked(limitedApi.get);

describe('fileListService fulltext search', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('uses the fulltext endpoint for non-empty search queries and keeps snippets', async () => {
    limitedGet.mockResolvedValueOnce({
      data: {
        query: 'invoice',
        count: 1,
        files: [
          {
            file: {
              id: 'file-1',
              filename: 'stored-name.txt',
              original_filename: 'receipt.txt',
              file_size: 128,
              mime_type: 'text/plain',
              category: null,
              folder_id: null,
              created_at: '2026-05-14T00:00:00Z',
              deleted_at: null,
            },
            score: 3.5,
            snippet: 'paid invoice number 42',
            match_source: 'content',
          },
        ],
      },
      status: 200,
      statusText: 'OK',
      headers: {},
      config: { headers: new AxiosHeaders() },
    });

    const result = await fileListService.listFiles({
      search: 'invoice',
      limit: 20,
      folder_id: 'folder-1',
      mime_type: 'text/plain',
    });

    expect(limitedGet).toHaveBeenCalledWith(
      '/api/files/search/fulltext?q=invoice&limit=20&folder_id=folder-1&mime_type=text%2Fplain',
    );
    expect(result.total).toBe(1);
    expect(result.files[0].search_snippet).toBe('paid invoice number 42');
    expect(result.files[0].match_source).toBe('content');
  });
});
