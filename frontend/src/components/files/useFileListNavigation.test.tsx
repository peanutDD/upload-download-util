import React from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useFileList } from "./useFileList";
import { useFiles } from "../../hooks/files/useFiles";
import { useFolderContents } from "../../hooks/folders/useFolders";

vi.mock("../../hooks/files/useFiles", () => ({
  useFiles: vi.fn(),
}));

vi.mock("../../hooks/folders/useFolders", () => ({
  useFolderContents: vi.fn(),
}));

vi.mock("../../hooks/files/useFileActions", () => ({
  useFileActions: () => ({
    deleteLoading: false,
    batchDownloading: false,
    handleDelete: vi.fn(),
    handleBatchDelete: vi.fn(),
    executeDelete: vi.fn(),
    handleRenameFolderSubmit: vi.fn(),
    handleRenameFileSubmit: vi.fn(),
    handleDownload: vi.fn(),
    handleBatchDownload: vi.fn(),
    handleDropOnBreadcrumb: vi.fn(),
  }),
}));

const scrollToMock = vi.fn();
const refetch = vi.fn();

function createWrapper(initialEntries: string[]) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  return function Wrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <MemoryRouter initialEntries={initialEntries}>{children}</MemoryRouter>
      </QueryClientProvider>
    );
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  sessionStorage.clear();
  localStorage.clear();

  Object.defineProperty(window, "scrollY", {
    configurable: true,
    value: 215,
  });
  Object.defineProperty(window, "scrollTo", {
    configurable: true,
    value: scrollToMock,
  });
  vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => {
    callback(0);
    return 0;
  });

  vi.mocked(useFiles).mockReturnValue({
    data: { pages: [{ files: [], total: 0 }] },
    fetchNextPage: vi.fn(),
    hasNextPage: false,
    isFetchingNextPage: false,
    isLoading: false,
    isFetching: false,
    refetch,
  } as unknown as ReturnType<typeof useFiles>);

  vi.mocked(useFolderContents).mockImplementation((folderId) => ({
    data: {
      current: null,
      path: folderId ? [{
        id: folderId,
        name: "Saved folder",
        parent_id: null,
        created_at: "2026-05-05T00:00:00.000Z",
        updated_at: "2026-05-05T00:00:00.000Z",
      }] : [],
      folders: [],
    },
    isLoading: false,
    refetch,
  }) as unknown as ReturnType<typeof useFolderContents>);
});

describe("useFileList navigation scroll restoration", () => {
  it("restores a saved folder scroll position when entering that folder", async () => {
    sessionStorage.setItem("fileListScroll:folder-1:created_at_desc:all:", "640");

    const { result } = renderHook(() => useFileList(), {
      wrapper: createWrapper(["/files"]),
    });
    await waitFor(() => expect(result.current.currentFolderId).toBeNull());
    scrollToMock.mockClear();

    act(() => {
      result.current.navigateToFolder("folder-1");
    });

    await waitFor(() =>
      expect(scrollToMock).toHaveBeenLastCalledWith({
        top: 640,
        left: 0,
        behavior: "auto",
      }),
    );
  });

  it("persists the active file list scroll position before a page refresh", async () => {
    renderHook(() => useFileList(), { wrapper: createWrapper(["/files?folder=folder-1"]) });

    window.dispatchEvent(new Event("pagehide"));

    expect(
      sessionStorage.getItem("fileListScroll:folder-1:created_at_desc:all:"),
    ).toBe("215");
  });
});
