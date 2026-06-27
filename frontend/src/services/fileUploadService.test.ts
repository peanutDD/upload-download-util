import { beforeEach, describe, expect, it, vi } from 'vitest';
import api from './api';
import { fileUploadService } from './fileUploadService';
import { sha256BlobHex, sha256FileHex } from '../utils/sha256';
import { CHUNKED_UPLOAD } from '../constants';

vi.mock('./api', () => ({
  default: {
    post: vi.fn(),
    put: vi.fn(),
    get: vi.fn(),
    delete: vi.fn(),
  },
}));

vi.mock('../utils/sha256', () => ({
  sha256FileHex: vi.fn(),
  sha256BlobHex: vi.fn(),
}));

vi.mock('../utils/telemetry', () => ({
  trackError: vi.fn(),
  trackEvent: vi.fn(),
}));

const mockedApi = vi.mocked(api);
const apiPost = mockedApi.post as unknown as ReturnType<typeof vi.fn>;
const apiGet = mockedApi.get as unknown as ReturnType<typeof vi.fn>;
const apiPut = mockedApi.put as unknown as ReturnType<typeof vi.fn>;
const apiDelete = mockedApi.delete as unknown as ReturnType<typeof vi.fn>;
const mockedSha256 = sha256FileHex as unknown as ReturnType<typeof vi.fn>;
const mockedBlobSha256 = sha256BlobHex as unknown as ReturnType<typeof vi.fn>;

function makeFile(name: string, type: string, body = 'hello', lastModified = 123): File {
  return new File([body], name, { type, lastModified });
}

function flushPromises(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

describe('fileUploadService upload orchestration', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mockedSha256.mockResolvedValue('a'.repeat(64));
    mockedBlobSha256.mockResolvedValue('b'.repeat(64));
    localStorage.clear();
  });

  it('passes AbortSignal through hash, instant check, and normal upload requests', async () => {
    const controller = new AbortController();
    const file = makeFile('note.txt', 'text/plain');

    apiPost
      .mockResolvedValueOnce({ status: 200, data: { instant: false } })
      .mockResolvedValueOnce({
        data: {
          file: {
            id: 'file-1',
            filename: 'note.txt',
          },
        },
      });

    await fileUploadService.uploadFileWithInstant(
      file,
      undefined,
      null,
      { signal: controller.signal },
    );

    expect(mockedSha256).toHaveBeenCalledWith(file, controller.signal);
    expect(apiPost).toHaveBeenNthCalledWith(
      1,
      '/api/files/upload/instant',
      expect.objectContaining({ filename: 'note.txt' }),
      expect.objectContaining({ signal: controller.signal }),
    );
    expect(apiPost).toHaveBeenNthCalledWith(
      2,
      '/api/files/upload',
      expect.any(FormData),
      expect.objectContaining({ signal: controller.signal }),
    );
  });

  it('uses the inferred MIME type for normal uploads when the browser reports octet-stream', async () => {
    const file = makeFile('finder-image.png', 'application/octet-stream', 'png-bytes');

    apiPost.mockResolvedValueOnce({
      data: {
        file: {
          id: 'file-1',
          filename: 'finder-image.png',
        },
      },
    });

    await fileUploadService.uploadFile(file);

    const formData = apiPost.mock.calls[0][1] as FormData;
    const uploadedFile = formData.get('file') as File;
    expect(uploadedFile.type).toBe('image/png');
    expect(uploadedFile.name).toBe('finder-image.png');
    expect(uploadedFile.lastModified).toBe(file.lastModified);
  });

  it('aborts the server chunked upload session when a queued upload is cancelled after init', async () => {
    const controller = new AbortController();
    const file = makeFile('movie.mp4', 'video/mp4', 'video');

    apiPost.mockImplementationOnce(async () => {
      controller.abort();
      return {
        data: {
          upload_id: 'upload-1',
          chunk_size: 2,
          total_parts: 3,
        },
      };
    });
    apiGet.mockResolvedValue({ data: { uploaded_parts: [], total_parts: 3 } });
    apiDelete.mockResolvedValue({ data: {} });

    await expect(
      fileUploadService.uploadFileChunked(file, undefined, null, {
        signal: controller.signal,
      }),
    ).rejects.toMatchObject({ name: 'AbortError' });

    expect(apiDelete).toHaveBeenCalledWith(
      '/api/files/upload/chunked/upload-1/abort',
    );
  });

  it('sends a SHA-256 header for every uploaded chunk', async () => {
    const file = makeFile('movie.mp4', 'video/mp4', 'abcde');

    apiPost
      .mockResolvedValueOnce({
        data: {
          upload_id: 'upload-1',
          chunk_size: 5,
          total_parts: 1,
        },
      })
      .mockResolvedValueOnce({
        data: {
          file: {
            id: 'file-1',
            filename: 'movie.mp4',
          },
        },
      });
    apiGet.mockResolvedValue({ data: { uploaded_parts: [], total_parts: 1 } });
    apiPut.mockResolvedValue({ data: { ok: true } });

    await fileUploadService.uploadFileChunked(file);

    expect(mockedBlobSha256).toHaveBeenCalledWith(expect.any(Blob), undefined);
    expect(apiPut).toHaveBeenCalledWith(
      '/api/files/upload/chunked/upload-1/chunk?part=1',
      expect.any(Blob),
      expect.objectContaining({
        headers: {
          'Content-Type': 'application/octet-stream',
          'X-Part-SHA256': 'b'.repeat(64),
        },
      }),
    );
  });

  it('starts the next chunk as soon as one parallel lane finishes', async () => {
    const parallelChunks = CHUNKED_UPLOAD.PARALLEL_CHUNKS;
    const totalParts = parallelChunks + 2;
    const uploadedParts = new Set<number>();
    const pendingPuts: Array<{ part: number; resolve: () => void }> = [];
    const file = makeFile('movie.mp4', 'video/mp4', 'x'.repeat(totalParts));

    apiPost
      .mockResolvedValueOnce({
        data: {
          upload_id: 'upload-1',
          chunk_size: 1,
          total_parts: totalParts,
        },
      })
      .mockResolvedValueOnce({
        data: {
          file: {
            id: 'file-1',
            filename: 'movie.mp4',
          },
        },
      });
    apiGet.mockImplementation(async () => ({
      data: {
        uploaded_parts: Array.from(uploadedParts),
        total_parts: totalParts,
      },
    }));
    apiPut.mockImplementation((url: string) => {
      const part = Number(new URL(url, 'http://localhost').searchParams.get('part'));
      return new Promise((resolve) => {
        pendingPuts.push({
          part,
          resolve: () => {
            uploadedParts.add(part);
            resolve({ data: { ok: true } });
          },
        });
      });
    });

    const uploadPromise = fileUploadService.uploadFileChunked(file);
    await vi.waitFor(() => expect(apiPut).toHaveBeenCalledTimes(parallelChunks));

    pendingPuts[0].resolve();
    await flushPromises();

    expect(apiPut).toHaveBeenCalledTimes(parallelChunks + 1);

    while (pendingPuts.length > 0) {
      const next = pendingPuts.shift();
      next?.resolve();
      await flushPromises();
    }
    await uploadPromise;
  });

  it('resumes a persisted chunked upload session instead of creating a new one', async () => {
    const file = makeFile('movie.mp4', 'video/mp4', 'abcde', 456);
    const contentSha256 = 'a'.repeat(64);
    localStorage.setItem(
      [
        'file-storage:chunked-upload:v1',
        contentSha256,
        file.size,
        file.lastModified,
        'root',
        'video/mp4',
        file.name,
      ].join(':'),
      JSON.stringify({
        uploadId: 'upload-existing',
        chunkSize: 5,
        totalParts: 1,
        fileName: file.name,
        fileSize: file.size,
        fileLastModified: file.lastModified,
        mimeType: 'video/mp4',
        folderId: null,
        contentSha256,
        updatedAt: 1,
      }),
    );
    apiGet.mockResolvedValue({ data: { uploaded_parts: [1], total_parts: 1 } });
    apiPost.mockResolvedValueOnce({
      data: {
        file: {
          id: 'file-1',
          filename: 'movie.mp4',
        },
      },
    });

    await fileUploadService.uploadFileChunked(file);

    expect(apiPost).not.toHaveBeenCalledWith(
      '/api/files/upload/chunked/init',
      expect.anything(),
      expect.anything(),
    );
    expect(apiPost).toHaveBeenCalledWith(
      '/api/files/upload/chunked/upload-existing/complete',
      expect.objectContaining({ filename: 'movie.mp4' }),
      expect.anything(),
    );
    expect(apiPut).not.toHaveBeenCalled();
  });

  it('keeps a chunked session after retryable failures so the next attempt can resume', async () => {
    vi.useFakeTimers();
    const file = makeFile('movie.mp4', 'video/mp4', 'abcde');

    try {
      apiPost.mockResolvedValueOnce({
        data: {
          upload_id: 'upload-1',
          chunk_size: 5,
          total_parts: 1,
        },
      });
      apiGet.mockResolvedValue({ data: { uploaded_parts: [], total_parts: 1 } });
      apiPut.mockRejectedValue(Object.assign(new Error('Network Error'), { response: undefined }));

      const uploadPromise = expect(fileUploadService.uploadFileChunked(file)).rejects.toThrow(
        'Network Error',
      );
      await vi.runAllTimersAsync();
      await uploadPromise;

      expect(apiDelete).not.toHaveBeenCalled();
      expect(localStorage.length).toBe(1);
    } finally {
      vi.useRealTimers();
    }
  });
});
