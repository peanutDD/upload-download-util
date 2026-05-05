import { useCallback, useMemo, useRef, useEffect, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useLocation, useSearchParams } from 'react-router-dom';
import type { FileMetadata } from '../../types/files';
import type { Folder } from '../../types/folders';
import { MIME_FILTER_FOLDERS } from '../../constants';
import { useFileFilters } from '../../hooks/files/useFileFilters';
import { useFileSelection } from '../../hooks/files/useFileSelection';
import { FILE_TYPE_LABELS } from './fileTypeLabels';
import { groupFilesInWorker } from '../../utils/workerPool';
import { useFiles } from '../../hooks/files/useFiles';
import { useFolderContents } from '../../hooks/folders/useFolders';
import { useFileUI } from '../../hooks/files/useFileUI';
import { useFileActions } from '../../hooks/files/useFileActions';

// 重新导出 FILE_TYPE_LABELS 以保持向后兼容
export { FILE_TYPE_LABELS };

const GROUP_FILES_WORKER_THRESHOLD = 50;

function getTypeKey(mime: string): string {
  if (mime.toLowerCase().startsWith('image/gif')) return 'gif';
  if (mime.startsWith('image/')) return 'image';
  if (mime.startsWith('video/')) return 'video';
  if (mime.startsWith('audio/')) return 'audio';
  if (mime === 'application/pdf') return 'application/pdf';
  if (mime.startsWith('text/')) return 'text';
  if (mime === 'application/zip' || mime === 'application/x-zip-compressed') return 'application/zip';
  if (mime.startsWith('application/')) return 'application';
  return 'other';
}

function getTimeGroupInfo(dateStr: string): { key: string; label: string; sortKey: number } {
  const date = new Date(dateStr);
  const year = date.getFullYear();
  const month = date.getMonth() + 1;
  const day = date.getDate();
  const key = `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
  const monthNames = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  const label = `${monthNames[month - 1]} ${day}, ${year}`;
  const sortKey = year * 10000 + month * 100 + day;
  return { key, label, sortKey };
}

function useFileGroupingWithIcons(
  files: FileMetadata[],
  isGroupByType: boolean,
  shouldBuildIndex: boolean
) {
  const typeOrderForWorker = useMemo(
    () => Object.fromEntries(Object.entries(FILE_TYPE_LABELS).map(([k, v]) => [k, v.order])),
    []
  );

  const [workerGrouped, setWorkerGrouped] = useState<
    Array<{ key: string; order: number; files: FileMetadata[] }> | null
  >(null);

  const requestIdRef = useRef(0);

  useEffect(() => {
    if (files.length <= GROUP_FILES_WORKER_THRESHOLD || !isGroupByType) {
      requestIdRef.current += 1;
      return;
    }

    const currentRequestId = ++requestIdRef.current;

    groupFilesInWorker(files as Parameters<typeof groupFilesInWorker>[0], typeOrderForWorker)
      .then((result) => {
        if (requestIdRef.current === currentRequestId) {
          setWorkerGrouped(result as Array<{ key: string; order: number; files: FileMetadata[] }>);

        }
      })
      .catch(() => {
        if (requestIdRef.current === currentRequestId) {
          setWorkerGrouped(null);
        }
      });

    return () => {
      requestIdRef.current = currentRequestId + 1;
    };
  }, [files, isGroupByType, typeOrderForWorker]);

  const memoGrouped = useMemo(() => {
    if (!isGroupByType) return null;

    const groups = new Map<string, FileMetadata[]>();
    files.forEach((file) => {
      const key = getTypeKey(file.mime_type);
      if (!groups.has(key)) {
        groups.set(key, []);
      }
      groups.get(key)!.push(file);
    });

    return Array.from(groups.entries())
      .map(([key, groupFiles]) => ({
        key,
        ...(FILE_TYPE_LABELS[key] ?? FILE_TYPE_LABELS.other),
        files: groupFiles,
      }))
      .sort((a, b) => a.order - b.order);
  }, [files, isGroupByType]);

  const groupedFiles = useMemo(() => {
    if (!isGroupByType) return null;
    if (files.length > GROUP_FILES_WORKER_THRESHOLD && workerGrouped) {
      return workerGrouped.map(({ key, files: groupFiles }) => ({
        key,
        ...(FILE_TYPE_LABELS[key] ?? FILE_TYPE_LABELS.other),
        files: groupFiles,
      }));
    }
    return memoGrouped;
  }, [isGroupByType, files.length, workerGrouped, memoGrouped]);

  const displayFiles = useMemo(() => {
    if (!isGroupByType || !groupedFiles) return files;
    return groupedFiles.flatMap((group) => group.files);
  }, [files, isGroupByType, groupedFiles]);

  const emptyIndex = useMemo(() => new Map<string, number>(), []);
  const displayFileIndexById = useMemo(() => {
    if (!shouldBuildIndex) return emptyIndex;
    const m = new Map<string, number>();
    for (let i = 0; i < displayFiles.length; i += 1) {
      m.set(displayFiles[i]!.id, i);
    }
    return m;
  }, [displayFiles, shouldBuildIndex, emptyIndex]);

  return {
    groupedFiles,
    displayFiles,
    displayFileIndexById,
  };
}

function useTimeGrouping(files: FileMetadata[], isGroupByTime: boolean) {
  const timeGroupedFiles = useMemo(() => {
    if (!isGroupByTime) return null;

    const groups = new Map<string, { label: string; sortKey: number; files: FileMetadata[] }>();

    for (const file of files) {
      const createdAt = file.created_at;
      const { key, label, sortKey } = getTimeGroupInfo(createdAt);
      const existing = groups.get(key);
      if (existing) {
        existing.files.push(file);
      } else {
        groups.set(key, { label, sortKey, files: [file] });
      }
    }

    return Array.from(groups.entries())
      .map(([key, { label, sortKey, files: groupFiles }]) => ({
        key,
        label,
        sortKey,
        files: groupFiles,
      }))
      .sort((a, b) => b.sortKey - a.sortKey);
  }, [files, isGroupByTime]);

  const displayFilesForTime = useMemo(() => {
    if (!isGroupByTime || !timeGroupedFiles) return files;
    return timeGroupedFiles.flatMap((group) => group.files);
  }, [files, isGroupByTime, timeGroupedFiles]);

  return {
    timeGroupedFiles,
    displayFilesForTime,
  };
}

function useTimeGroupingMixed(
  files: FileMetadata[],
  folders: Folder[],
  isGroupByTime: boolean
) {
  const timeGroupedItems = useMemo(() => {
    if (!isGroupByTime) return null;
    const groups = new Map<
      string,
      { label: string; sortKey: number; files: FileMetadata[]; folders: Folder[] }
    >();

    for (const folder of folders) {
      const { key, label, sortKey } = getTimeGroupInfo(folder.created_at);
      const existing = groups.get(key);
      if (existing) {
        existing.folders.push(folder);
      } else {
        groups.set(key, { label, sortKey, files: [], folders: [folder] });
      }
    }

    for (const file of files) {
      const { key, label, sortKey } = getTimeGroupInfo(file.created_at);
      const existing = groups.get(key);
      if (existing) {
        existing.files.push(file);
      } else {
        groups.set(key, { label, sortKey, files: [file], folders: [] });
      }
    }

    const compareName = (a: string, b: string) =>
      a.localeCompare(b, undefined, { numeric: true, sensitivity: 'base' });
    const getTime = (v: string) => {
      const t = Date.parse(v);
      return Number.isFinite(t) ? t : 0;
    };

    return Array.from(groups.entries())
      .map(([key, { label, sortKey, files: gFiles, folders: gFolders }]) => {
        const items = [
          ...gFolders.map((folder) => ({ type: 'folder' as const, folder })),
          ...gFiles.map((file) => ({ type: 'file' as const, file })),
        ].sort((a, b) => {
          const at = a.type === 'folder' ? getTime(a.folder.created_at) : getTime(a.file.created_at);
          const bt = b.type === 'folder' ? getTime(b.folder.created_at) : getTime(b.file.created_at);
          const r = bt - at;
          if (r !== 0) return r;
          const an = a.type === 'folder' ? a.folder.name : a.file.original_filename;
          const bn = b.type === 'folder' ? b.folder.name : b.file.original_filename;
          return compareName(an, bn);
        });
        return { key, label, sortKey, files: gFiles, folders: gFolders, items };
      })
      .sort((a, b) => b.sortKey - a.sortKey);
  }, [files, folders, isGroupByTime]);

  return { timeGroupedItems };
}

export function useFileList() {
  const queryClient = useQueryClient();
  const [searchParams, setSearchParams] = useSearchParams();
  const currentFolderId = searchParams.get('folder') || null;

  // 使用抽取的过滤器 Hook
  const {
    search,
    mimeType,
    sortBy,
    debouncedSearch,
    sortField,
    sortOrder,
    isGroupByType,
    isGroupByTime,
    handleSearchChange,
    handleMimeTypeChange,
    handleSortChange,
  } = useFileFilters();

  // 使用 TanStack Query 获取文件
  const {
    data: filesData,
    fetchNextPage: loadMore,
    hasNextPage: hasMore,
    isFetchingNextPage: loadingMore,
    isLoading: loadingFiles,
    isFetching: isFetchingFiles,
    refetch: refetchFiles,
  } = useFiles({
    search: debouncedSearch || undefined,
    mime_type: mimeType || undefined,
    folder_id: currentFolderId,
    sort_by: sortField,
    sort_order: sortOrder,
  });

  const files = useMemo(() => {
    const flat = filesData?.pages.flatMap((page) => page.files) ?? [];
    if (flat.length <= 1) return flat;
    const seen = new Set<string>();
    const deduped: FileMetadata[] = [];
    for (const file of flat) {
      if (seen.has(file.id)) continue;
      seen.add(file.id);
      deduped.push(file);
    }
    return deduped;
  }, [filesData]);
  const totalItems = filesData?.pages[0]?.total ?? 0;

  const sortedFiles = useMemo(() => {
    if (files.length <= 1) return files;
    if (!sortBy.startsWith('filename_')) return files;
    const dir = sortBy.endsWith('_asc') ? 1 : -1;
    const compareName = (a: string, b: string) =>
      a.localeCompare(b, undefined, { numeric: true, sensitivity: 'base' });
    const leadingZeroScore = (s: string) => {
      let score = 0;
      let i = 0;
      while (i < s.length) {
        const c = s.charCodeAt(i);
        const isDigit = c >= 48 && c <= 57;
        if (!isDigit) {
          i += 1;
          continue;
        }
        const start = i;
        while (i < s.length) {
          const cc = s.charCodeAt(i);
          if (cc < 48 || cc > 57) break;
          i += 1;
        }
        const raw = s.slice(start, i);
        const stripped = raw.replace(/^0+/, '');
        const normalized = stripped === '' ? '0' : stripped;
        score += raw.length - normalized.length;
      }
      return score;
    };
    const getTime = (v: string) => {
      const t = Date.parse(v);
      return Number.isFinite(t) ? t : 0;
    };
    const list = [...files];
    list.sort((a, b) => {
      const r = compareName(a.original_filename, b.original_filename);
      if (r !== 0) return r * dir;
      const z = leadingZeroScore(a.original_filename) - leadingZeroScore(b.original_filename);
      if (z !== 0) return z * dir;
      const tr = getTime(a.created_at) - getTime(b.created_at);
      if (tr !== 0) return tr;
      const cr = a.original_filename.localeCompare(b.original_filename, undefined, {
        numeric: false,
        sensitivity: 'base',
      });
      if (cr !== 0) return cr * dir;
      return a.id.localeCompare(b.id);
    });
    return list;
  }, [files, sortBy]);

  // 使用 TanStack Query 获取文件夹内容
  const {
    data: folderContents,
    isLoading: loadingFolders,
    refetch: refetchFolders,
  } = useFolderContents(currentFolderId);

  const folders = useMemo(() => folderContents?.folders ?? [], [folderContents]);
  const folderPath = useMemo(() => folderContents?.path ?? [], [folderContents]);

  // 使用抽取的选择状态 Hook
  const {
    selectedFiles,
    selectedFolders,
    selectedFileIds,
    selectedFolderIds,
    allFilesSelected,
    toggleSelectAll,
    clearSelection,
    setSelectedFiles,
    setSelectedFolders,
  } = useFileSelection(files, folders);

  // UI State Hook
  const {
    previewFile,
    setPreviewFile,
    shareFile,
    setShareFile,
    showBatchShare,
    setShowBatchShare,
    batchShareFileIds,
    setBatchShareFileIds,
    showBatchMove,
    setShowBatchMove,
    showCreateFolder,
    setShowCreateFolder,
    renamingFolder,
    setRenamingFolder,
    renamingFile,
    setRenamingFile,
    deleteConfirm,
    setDeleteConfirm,
    error,
    setError,
    clearError,
  } = useFileUI();

  const lastSelectionScopeRef = useRef<{ sortBy: string; mimeType: string } | null>(null);

  // 使用带 icon 的分组 Hook
  const {
    groupedFiles,
    displayFiles,
  } = useFileGroupingWithIcons(sortedFiles, isGroupByType, previewFile !== null);

  // 使用时间分组 Hook
  const {
    timeGroupedFiles,
    displayFilesForTime,
  } = useTimeGrouping(sortedFiles, isGroupByTime);

  const finalDisplayFiles = useMemo(
    () => (isGroupByTime ? displayFilesForTime : displayFiles),
    [isGroupByTime, displayFilesForTime, displayFiles]
  );

  const finalDisplayFileIndexById = useMemo(() => {
    if (previewFile === null) return new Map<string, number>();
    const m = new Map<string, number>();
    for (let i = 0; i < finalDisplayFiles.length; i += 1) {
      m.set(finalDisplayFiles[i]!.id, i);
    }
    return m;
  }, [finalDisplayFiles, previewFile]);

  // Actions Hook
  const {
    deleteLoading,
    batchDownloading,
    handleDelete,
    handleBatchDelete,
    executeDelete: wrappedExecuteDelete,
    handleRenameFolderSubmit,
    handleRenameFileSubmit,
    handleDownload,
    handleBatchDownload,
    handleDropOnBreadcrumb,
  } = useFileActions({
    files,
    selectedFiles,
    selectedFolders,
    selectedFileIds,
    selectedFolderIds,
    setSelectedFiles,
    setSelectedFolders,
    setError,
    setDeleteConfirm,
    deleteConfirm,
    setRenamingFolder,
    setRenamingFile,
    refetchFiles,
    refetchFolders,
  });

  // ── 即时 DOM 隐藏：确认删除时立刻把卡片从页面移除 ─────────────────────────────
  // 不依赖 React Query 缓存更新时机，用本地 state 直接驱动 filter，解决
  // "第二次删除 DOM 不立刻更新"的 bug。
  const [pendingDeleteFileIds, setPendingDeleteFileIds] = useState<Set<string>>(new Set());
  const [pendingDeleteFolderIds, setPendingDeleteFolderIds] = useState<Set<string>>(new Set());

  // 若删除后文件重新出现在 files 里（代表删除失败/被回滚），从 pending 中移除，让卡片还原
  useEffect(() => {
    if (pendingDeleteFileIds.size === 0) return;
    const currentIds = new Set(files.map((f) => f.id));
    const reappeared = [...pendingDeleteFileIds].filter((id) => currentIds.has(id));
    if (reappeared.length === 0) return;

    let cancelled = false;
    queueMicrotask(() => {
      if (cancelled) return;
      setPendingDeleteFileIds((prev) => {
        const n = new Set(prev);
        reappeared.forEach((id) => n.delete(id));
        return n;
      });
    });

    return () => {
      cancelled = true;
    };
  }, [files, pendingDeleteFileIds]);

  // 包装 executeDelete：先把 ID 加入 pending（立刻触发 re-render 隐藏卡片），再执行删除
  const executeDeleteWithHide = useCallback(async () => {
    const snap = deleteConfirm;
    if (snap?.type === 'file' && snap.id) {
      setPendingDeleteFileIds((prev) => new Set([...prev, snap.id!]));
    } else if (snap?.type === 'folder' && snap.id) {
      setPendingDeleteFolderIds((prev) => new Set([...prev, snap.id!]));
    } else if (snap?.type === 'batch') {
      if (selectedFileIds.length > 0)
        setPendingDeleteFileIds((prev) => new Set([...prev, ...selectedFileIds]));
      if (selectedFolderIds.length > 0)
        setPendingDeleteFolderIds((prev) => new Set([...prev, ...selectedFolderIds]));
    }
    await wrappedExecuteDelete();
  }, [deleteConfirm, wrappedExecuteDelete, selectedFileIds, selectedFolderIds]);

  // 应用 pending 过滤，给渲染层用的输出变量
  const outFiles = useMemo(
    () =>
      pendingDeleteFileIds.size > 0
        ? files.filter((f) => !pendingDeleteFileIds.has(f.id))
        : files,
    [files, pendingDeleteFileIds],
  );

  const scrollStorageContextRef = useRef({ currentFolderId, debouncedSearch, mimeType, sortBy });
  scrollStorageContextRef.current = {
    currentFolderId,
    debouncedSearch,
    mimeType,
    sortBy,
  };

  const getScrollStorageKey = useCallback((folderId: string | null) => {
    const {
      debouncedSearch: storedSearch,
      mimeType: storedMimeType,
      sortBy: storedSortBy,
    } = scrollStorageContextRef.current;
    const folderKey = folderId ?? 'root';
    const q = (storedSearch ?? '').trim();
    const mimeKey = storedMimeType || 'all';
    return `fileListScroll:${folderKey}:${storedSortBy}:${mimeKey}:${q}`;
  }, []);

  const location = useLocation();
  const lastScrollAppliedKeyRef = useRef<string | null>(null);
  const pendingScrollRestoreStorageKeyRef = useRef<string | null>(null);

  const saveScrollPosition = useCallback(
    (folderId: string | null = scrollStorageContextRef.current.currentFolderId, force = false) => {
      try {
        const key = getScrollStorageKey(folderId);
        if (!force && pendingScrollRestoreStorageKeyRef.current === key) {
          return;
        }
        const y = Math.max(0, Math.round(window.scrollY || 0));
        sessionStorage.setItem(key, String(y));
      } catch {
        /* ignore */
      }
    },
    [getScrollStorageKey],
  );

  const navigateToFolder = useCallback((folderId: string | null) => {
    saveScrollPosition(currentFolderId, true);
    pendingScrollRestoreStorageKeyRef.current = getScrollStorageKey(folderId);
    setSearchParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        if (folderId) {
          next.set('folder', folderId);
        } else {
          next.delete('folder');
        }
        return next;
      },
      { replace: false }
    );
    setSelectedFiles(new Set());
    setSelectedFolders(new Set());
  }, [
    setSearchParams,
    setSelectedFiles,
    setSelectedFolders,
    currentFolderId,
    saveScrollPosition,
    getScrollStorageKey,
  ]);

  useEffect(() => {
    let frame: number | null = null;

    const saveNow = () => {
      if (frame !== null) {
        cancelAnimationFrame(frame);
        frame = null;
      }
      saveScrollPosition();
    };

    const scheduleSave = () => {
      if (frame !== null) return;
      frame = requestAnimationFrame(() => {
        frame = null;
        saveScrollPosition();
      });
    };

    const saveWhenHidden = () => {
      if (document.visibilityState === 'hidden') {
        saveNow();
      }
    };

    window.addEventListener('scroll', scheduleSave, { passive: true });
    window.addEventListener('pagehide', saveNow);
    document.addEventListener('visibilitychange', saveWhenHidden);

    return () => {
      window.removeEventListener('scroll', scheduleSave);
      window.removeEventListener('pagehide', saveNow);
      document.removeEventListener('visibilitychange', saveWhenHidden);
      saveNow();
    };
  }, [saveScrollPosition]);

  useEffect(() => {
    if (loadingFiles || loadingFolders) return;
    const key = getScrollStorageKey(currentFolderId);
    const scrollAppliedKey = `${location.key}:${key}`;
    if (lastScrollAppliedKeyRef.current === scrollAppliedKey) return;
    lastScrollAppliedKeyRef.current = scrollAppliedKey;
    pendingScrollRestoreStorageKeyRef.current = key;

    let cancelled = false;
    const clearPendingScrollRestore = () => {
      if (pendingScrollRestoreStorageKeyRef.current === key) {
        pendingScrollRestoreStorageKeyRef.current = null;
      }
    };
    const restoreScroll = (top: number) => {
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          if (cancelled) return;
          window.scrollTo({ top, left: 0, behavior: 'auto' });
          requestAnimationFrame(() => {
            if (!cancelled) clearPendingScrollRestore();
          });
        });
      });
    };

    let raw: string | null = null;
    try {
      raw = sessionStorage.getItem(key);
    } catch {
      raw = null;
    }
    if (!raw) {
      restoreScroll(0);
      return () => {
        cancelled = true;
        clearPendingScrollRestore();
      };
    }
    const y = Number.parseInt(raw, 10);
    if (!Number.isFinite(y) || y < 0) {
      clearPendingScrollRestore();
      return;
    }
    restoreScroll(y);
    return () => {
      cancelled = true;
      clearPendingScrollRestore();
    };
  }, [
    currentFolderId,
    debouncedSearch,
    getScrollStorageKey,
    loadingFiles,
    loadingFolders,
    location.key,
    mimeType,
    sortBy,
  ]);

  const displayFolders = useMemo(() => {
    if (mimeType !== '' && mimeType !== MIME_FILTER_FOLDERS) return [];
    const q = debouncedSearch?.trim().toLowerCase();
    const filtered = q ? folders.filter((f) => f.name.toLowerCase().includes(q)) : folders;
    if (filtered.length <= 1) return filtered;

    const compareName = (a: string, b: string) =>
      a.localeCompare(b, undefined, { numeric: true, sensitivity: 'base' });
    const getTime = (v: string) => {
      const t = Date.parse(v);
      return Number.isFinite(t) ? t : 0;
    };

    const list = [...filtered];
    if (sortBy === 'time_group' || sortBy.startsWith('created_at_')) {
      const dir = sortBy.endsWith('_asc') ? 1 : -1;
      list.sort((a, b) => {
        const r = getTime(a.created_at) - getTime(b.created_at);
        if (r !== 0) return r * dir;
        return compareName(a.name, b.name);
      });
      return list;
    }
    if (sortBy.startsWith('filename_')) {
      const dir = sortBy.endsWith('_asc') ? 1 : -1;
      list.sort((a, b) => compareName(a.name, b.name) * dir);
      return list;
    }
    list.sort((a, b) => compareName(a.name, b.name));
    return list;
  }, [mimeType, folders, debouncedSearch, sortBy]);

  // outFolders 必须在 displayFolders 赋值之后计算，否则 displayFolders 还是初始空数组
  const outFolders = useMemo(
    () =>
      pendingDeleteFolderIds.size > 0
        ? displayFolders.filter((f) => !pendingDeleteFolderIds.has(f.id))
        : displayFolders,
    [displayFolders, pendingDeleteFolderIds],
  );

  // outFoldersRef 用于 pendingDeleteFolderIds 回滚检测的 useEffect
  const outFoldersRef = useRef<Folder[]>(displayFolders);

  useEffect(() => {
    outFoldersRef.current = displayFolders;
  }, [displayFolders]);

  useEffect(() => {
    if (pendingDeleteFolderIds.size === 0) return;
    const currentIds = new Set(outFoldersRef.current.map((f) => f.id));
    const reappeared = [...pendingDeleteFolderIds].filter((id) => currentIds.has(id));
    if (reappeared.length === 0) return;

    let cancelled = false;
    queueMicrotask(() => {
      if (cancelled) return;
      setPendingDeleteFolderIds((prev) => {
        const n = new Set(prev);
        reappeared.forEach((id) => n.delete(id));
        return n;
      });
    });

    return () => {
      cancelled = true;
    };
  }, [pendingDeleteFolderIds]);

  const { timeGroupedItems } = useTimeGroupingMixed(sortedFiles, displayFolders, isGroupByTime);

  const visibleFileIds = useMemo(() => new Set(finalDisplayFiles.map((file) => file.id)), [finalDisplayFiles]);
  const visibleFolderIds = useMemo(() => new Set(displayFolders.map((folder) => folder.id)), [displayFolders]);

  useEffect(() => {
    setSelectedFiles((prev) => {
      if (prev.size === 0) return prev;
      const next = new Set(Array.from(prev).filter((id) => visibleFileIds.has(id)));
      return next.size === prev.size ? prev : next;
    });
    setSelectedFolders((prev) => {
      if (prev.size === 0) return prev;
      const next = new Set(Array.from(prev).filter((id) => visibleFolderIds.has(id)));
      return next.size === prev.size ? prev : next;
    });
  }, [visibleFileIds, visibleFolderIds, setSelectedFiles, setSelectedFolders]);

  useEffect(() => {
    if (lastSelectionScopeRef.current === null) {
      lastSelectionScopeRef.current = { sortBy, mimeType };
      return;
    }
    if (
      lastSelectionScopeRef.current.sortBy !== sortBy ||
      lastSelectionScopeRef.current.mimeType !== mimeType
    ) {
      clearSelection();
      lastSelectionScopeRef.current = { sortBy, mimeType };
    }
  }, [sortBy, mimeType, clearSelection]);

  const handleShowBatchMove = useCallback(() => {
    if (selectedFiles.size === 0 && selectedFolders.size === 0) return;
    setShowBatchMove(true);
  }, [selectedFiles.size, selectedFolders.size, setShowBatchMove]);

  const handleShowBatchShare = useCallback(() => {
    if (selectedFiles.size === 0) return;
    setBatchShareFileIds(selectedFileIds);
    setShowBatchShare(true);
  }, [selectedFiles.size, selectedFileIds, setBatchShareFileIds, setShowBatchShare]);

  const refreshListsAfterMove = useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['files'] }),
      queryClient.invalidateQueries({ queryKey: ['folders', 'contents'] }),
    ]);
    await Promise.all([refetchFiles(), refetchFolders()]);
  }, [queryClient, refetchFiles, refetchFolders]);

  const addFolderToList = useCallback(() => {
    refetchFolders();
  }, [refetchFolders]);

  return {
    files: outFiles,
    folderPath,
    search,
    mimeType,
    sortBy,
    selectedFiles,
    selectedFolders,
    selectedFileIds,
    selectedFolderIds,
    currentFolderId,
    error,
    clearError,
    isLoading: loadingFiles || loadingFolders,
    isRevalidating: isFetchingFiles,
    totalItems,
    isGroupByType,
    isGroupByTime,
    groupedFiles,
    timeGroupedFiles,
    timeGroupedItems,
    displayFolders: outFolders,
    displayFiles: finalDisplayFiles,
    displayFileIndexById: finalDisplayFileIndexById,
    totalPages: Math.ceil(totalItems / 50),
    page: 1,
    hasMore,
    loadingMore,
    loadMore,
    allFilesSelected,
    handleSearchChange,
    handleMimeTypeChange,
    handleSortChange,
    toggleSelectAll,
    handleSelectFile: (fileId: string, selected: boolean) => {
        setSelectedFiles(prev => {
            const next = new Set(prev);
            if (selected) next.add(fileId); else next.delete(fileId);
            return next;
        });
    },
    handleSelectFolder: (folderId: string, selected: boolean) => {
        setSelectedFolders(prev => {
            const next = new Set(prev);
            if (selected) next.add(folderId); else next.delete(folderId);
            return next;
        });
    },
    handleRenameFolder: setRenamingFolder,
    handleRenameFile: setRenamingFile,
    handleRenameFolderSubmit,
    handleRenameFileSubmit,
    getOptimisticMoveRollback: () => () => {},
    navigateToFolder,
    handleDelete,
    handleDownload,
    handleBatchDownload,
    handleBatchDelete,
    handleShowBatchMove,
    handleShowBatchShare,
    handleDropOnBreadcrumb,
    loadFiles: refetchFiles,
    loadFolders: refetchFolders,
    refreshListsAfterMove,
    clearSelection,
    addFolderToList,
    previewFile,
    setPreviewFile,
    shareFile,
    setShareFile,
    showBatchShare,
    setShowBatchShare,
    batchShareFileIds,
    setBatchShareFileIds,
    showBatchMove,
    setShowBatchMove,
    showCreateFolder,
    setShowCreateFolder,
    renamingFolder,
    setRenamingFolder,
    renamingFile,
    setRenamingFile,
    deleteConfirm,
    deleteLoading,
    executeDelete: executeDeleteWithHide,
    setDeleteConfirm,
    batchDownloading,
  };
}
