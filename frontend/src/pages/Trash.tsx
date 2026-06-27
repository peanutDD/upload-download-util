import { useCallback, useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useLocation, useNavigate } from "react-router-dom";
import {
  ArchiveRestore,
  ArrowLeft,
  CheckSquare,
  Clock3,
  FileWarning,
  ShieldCheck,
  Trash2,
  XCircle,
} from "lucide-react";
import LazyThumbnail from "../components/files/preview/LazyThumbnail";
import PageLayout from "../components/layout/PageLayout";
import ConfirmDialog from "../components/common/dialog/ConfirmDialog";
import { EmptyState } from "../components/common/EmptyState";
import ErrorMessage from "../components/common/feedback/ErrorMessage";
import Spinner from "../components/common/feedback/Spinner";
import { fileService } from "../services/files";
import { useAuthStore } from "../store/authStore";
import { formatBytes, formatFileSizeCompact } from "../utils/format";
import { getErrorMessage } from "../utils/error";
import { resolveTrashReturnTarget } from "../utils/trashReturnTarget";
import { getMimeTypeLabel, isImageType } from "../utils/mimeType";
import { SelectionCheckbox } from "../components/common/form/SelectionCheckbox";
import type { FileMetadata } from "../types/files";
import "../components/files/list/FileListGlass.css";

type ConfirmState =
  | { type: "permanent"; file: FileMetadata }
  | { type: "batch-permanent" }
  | { type: "empty" }
  | null;

const EMPTY_FILES: FileMetadata[] = [];
const TRASH_RETENTION_DAYS = 30;
const RETENTION_MS = TRASH_RETENTION_DAYS * 24 * 60 * 60 * 1000;
const ONE_DAY_MS = 24 * 60 * 60 * 1000;
const TRASH_RETENTION_REFRESH_MS = 60 * 1000;

function getRetentionState(value?: string | null, now = Date.now()) {
  if (!value) {
    return {
      daysLeft: TRASH_RETENTION_DAYS,
      status: `${TRASH_RETENTION_DAYS}D LEFT`,
    };
  }

  const deletedAt = new Date(value).getTime();
  if (Number.isNaN(deletedAt)) {
    return {
      daysLeft: TRASH_RETENTION_DAYS,
      status: "PENDING",
    };
  }

  const elapsed = Math.min(Math.max(now - deletedAt, 0), RETENTION_MS);
  const daysLeft = Math.max(0, Math.ceil((RETENTION_MS - elapsed) / ONE_DAY_MS));

  return {
    daysLeft,
    status: daysLeft === 0 ? "PURGE QUEUED" : `${daysLeft}D LEFT`,
  };
}

function getCountdownClass(daysLeft: number) {
  if (daysLeft <= 3) return "text-[var(--trash-countdown-danger)]";
  if (daysLeft <= 10) return "text-[var(--trash-countdown-warn)]";
  return "text-[var(--trash-countdown-text)]";
}

function useTrashRetentionClock() {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const intervalId = window.setInterval(() => {
      setNow(Date.now());
    }, TRASH_RETENTION_REFRESH_MS);

    return () => window.clearInterval(intervalId);
  }, []);

  return now;
}

type TrashLocationState = {
  from?: string;
};

function getPlaceholderLabel(file: FileMetadata) {
  if (file.mime_type.includes("pdf")) return "PDF";
  if (file.mime_type.startsWith("video/")) return "VIDEO";
  if (file.mime_type.startsWith("audio/")) return "AUDIO";
  if (file.mime_type.includes("zip") || file.mime_type.includes("archive")) {
    return "ZIP";
  }
  return "FILE";
}

function TrashThumbnail({ file }: { file: FileMetadata }) {
  if (isImageType(file.mime_type)) {
    return (
      <LazyThumbnail
        fileId={file.id}
        mimeType={file.mime_type}
        filename={file.original_filename}
        className="h-full w-full rounded-none"
        priority="low"
      />
    );
  }

  return (
    <div className="flex h-full w-full flex-col items-center justify-center gap-[clamp(0.25rem,0.8vw,0.65rem)] bg-[image:var(--trash-placeholder-bg)] text-[var(--trash-placeholder-icon)]">
      <FileWarning className="h-[34%] w-[34%]" aria-hidden />
      <span className="max-w-[86%] truncate text-[clamp(0.48rem,0.8vw,0.78rem)] font-semibold uppercase tracking-[0.16em] text-[var(--trash-placeholder-text)]">
        {getPlaceholderLabel(file)}
      </span>
    </div>
  );
}

export default function Trash() {
  const navigate = useNavigate();
  const location = useLocation();
  const queryClient = useQueryClient();
  const user = useAuthStore((s) => s.user);
  const clearAuth = useAuthStore((s) => s.clearAuth);
  const [confirm, setConfirm] = useState<ConfirmState>(null);
  const [error, setError] = useState<string | null>(null);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const retentionNow = useTrashRetentionClock();

  const { data, isLoading, isFetching } = useQuery({
    queryKey: ["trash"],
    queryFn: () => fileService.listTrash(),
  });

  const files = data?.files ?? EMPTY_FILES;
  const selectedIdSet = useMemo(() => new Set(selectedIds), [selectedIds]);
  const selectedFiles = useMemo(
    () => files.filter((file) => selectedIdSet.has(file.id)),
    [files, selectedIdSet],
  );
  const visibleSelectedIds = useMemo(
    () => selectedFiles.map((file) => file.id),
    [selectedFiles],
  );
  const totalSize = useMemo(
    () => files.reduce((sum, file) => sum + file.file_size, 0),
    [files],
  );
  const selectedCount = selectedFiles.length;
  const allVisibleSelected = files.length > 0 && selectedCount === files.length;

  const invalidateFileViews = useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["trash"] }),
      queryClient.invalidateQueries({ queryKey: ["files"] }),
    ]);
  }, [queryClient]);

  const restoreMutation = useMutation({
    mutationFn: (fileId: string) => fileService.restoreFile(fileId),
    onSuccess: () => {
      void invalidateFileViews();
    },
    onError: (err) => setError(getErrorMessage(err, "还原失败")),
  });

  const batchRestoreMutation = useMutation({
    mutationFn: (fileIds: string[]) => {
      return fileService.batchRestoreFiles(fileIds);
    },
    onSuccess: (result) => {
      setSelectedIds([]);
      if (result.failed.length > 0) {
        setError(`${result.failed.length} 个文件还原失败，列表已刷新`);
      }
      void invalidateFileViews();
    },
    onError: (err) => setError(getErrorMessage(err, "批量还原失败")),
  });

  const permanentMutation = useMutation({
    mutationFn: (fileId: string) => fileService.permanentlyDeleteFile(fileId),
    onSuccess: () => {
      setConfirm(null);
      void invalidateFileViews();
    },
    onError: (err) => setError(getErrorMessage(err, "彻底删除失败")),
  });

  const batchPermanentMutation = useMutation({
    mutationFn: (fileIds: string[]) => {
      return fileService.batchPermanentlyDeleteFiles(fileIds);
    },
    onSuccess: (result) => {
      setConfirm(null);
      setSelectedIds([]);
      if (result.failed.length > 0) {
        setError(`${result.failed.length} 个文件彻底删除失败，列表已刷新`);
      }
      void invalidateFileViews();
    },
    onError: (err) => setError(getErrorMessage(err, "批量彻底删除失败")),
  });

  const emptyMutation = useMutation({
    mutationFn: () => fileService.emptyTrash(),
    onSuccess: () => {
      setConfirm(null);
      void invalidateFileViews();
    },
    onError: (err) => setError(getErrorMessage(err, "清空回收站失败")),
  });

  const handleLogout = useCallback(() => {
    clearAuth();
    navigate("/login");
  }, [clearAuth, navigate]);

  const handleBack = useCallback(() => {
    const state = location.state as TrashLocationState | null;
    navigate(resolveTrashReturnTarget(state?.from));
  }, [location.state, navigate]);

  const toggleSelected = useCallback((fileId: string) => {
    setSelectedIds((current) =>
      current.includes(fileId)
        ? current.filter((id) => id !== fileId)
        : [...current, fileId],
    );
  }, []);

  const toggleAllSelected = useCallback(() => {
    setSelectedIds(allVisibleSelected ? [] : files.map((file) => file.id));
  }, [allVisibleSelected, files]);

  const confirmTitle =
    confirm?.type === "empty"
      ? "清空回收站"
      : confirm?.type === "batch-permanent"
        ? "批量彻底删除"
        : "彻底删除文件";
  const confirmMessage =
    confirm?.type === "empty"
      ? "回收站内所有文件都会被彻底删除。"
      : confirm?.type === "batch-permanent"
        ? `选中的 ${selectedCount} 个文件都会被彻底删除。`
        : `「${confirm?.file.original_filename ?? ""}」将被彻底删除。`;
  const confirmText = confirm?.type === "empty" ? "彻底清空" : "彻底删除";
  const confirmLoading =
    confirm?.type === "empty"
      ? emptyMutation.isPending
      : confirm?.type === "batch-permanent"
        ? batchPermanentMutation.isPending
        : permanentMutation.isPending;

  return (
    <PageLayout
      title="Trash"
      username={user?.username}
      onLogout={handleLogout}
      useSolidBackground
      backgroundClassName="bg-[color:var(--filelist-page-bg)]"
      data-oid="trash-page"
    >
      <div className="fileListGlassScope space-y-[clamp(0.6rem,1.4vw,0.75rem)] sm:space-y-[clamp(0.75rem,2vw,1rem)]" data-testid="trash-shell">
        <div
          data-testid="trash-console"
          className="glass-panel glass-panel-toolbar fileListToolbarScale75 trashConsoleToolbar sticky top-[calc(clamp(4.75rem,7.6vw,6.25rem)+env(safe-area-inset-top)+0.75rem)] z-30 p-[clamp(0.6rem,1.4vw,0.75rem)]"
        >
          <div
            data-testid="trash-console-inner"
            className="trashConsoleInner relative flex flex-col gap-[clamp(0.65rem,1.6vw,0.9rem)] lg:flex-row lg:items-center lg:justify-between"
          >
            <div
              data-testid="trash-console-summary-row"
              className="trashConsoleSummaryRow inline-flex w-full max-w-full min-w-0 self-stretch flex-wrap items-center gap-x-[clamp(0.4rem,1vw,0.5rem)] gap-y-[clamp(0.2rem,0.7vw,0.25rem)] sm:gap-x-[clamp(0.6rem,1.4vw,0.75rem)]"
            >
              <h1 className="font-brand text-[clamp(0.82rem,1.25vw,1.05rem)] font-normal tracking-widest text-[var(--filelist-btn-text)]">
                Vault Console
              </h1>
              <span className="glass-chip border border-[var(--filelist-toolbar-btn-border)] bg-[var(--filelist-toolbar-btn-bg)] px-[clamp(0.3rem,0.8vw,0.375rem)] py-[clamp(0.0975rem,0.3vw,0.125rem)] text-[0.58rem] font-semibold uppercase tracking-[0.12em] text-[var(--filelist-btn-text)] sm:px-[clamp(0.4rem,1vw,0.5rem)] sm:text-[0.68rem]">
                {files.length} files
              </span>
              <span className="inline-flex items-center gap-[clamp(0.2rem,0.7vw,0.25rem)] text-[0.62rem] text-[var(--filelist-text-muted)] sm:text-[0.72rem]">
                <ShieldCheck className="h-[clamp(0.65rem,1.5vw,0.75rem)] w-[clamp(0.65rem,1.5vw,0.75rem)] text-[var(--filelist-btn-text)] sm:h-[clamp(0.75rem,1.8vw,0.875rem)] sm:w-[clamp(0.75rem,1.8vw,0.875rem)]" aria-hidden />
                {formatBytes(totalSize)}
              </span>
              <span className="inline-flex items-center gap-[clamp(0.2rem,0.7vw,0.25rem)] text-[0.62rem] text-[var(--filelist-text-muted)] sm:text-[0.72rem]">
                <Clock3 className="h-[clamp(0.65rem,1.5vw,0.75rem)] w-[clamp(0.65rem,1.5vw,0.75rem)] text-[var(--filelist-btn-text)] sm:h-[clamp(0.75rem,1.8vw,0.875rem)] sm:w-[clamp(0.75rem,1.8vw,0.875rem)]" aria-hidden />
                {TRASH_RETENTION_DAYS}d cleanup
                {isFetching && !isLoading ? " · refreshing" : ""}
              </span>
              {selectedCount > 0 && (
                <span className="glass-chip border border-[var(--filelist-toolbar-btn-border)] bg-[var(--filelist-toolbar-btn-bg)] px-[clamp(0.3rem,0.8vw,0.375rem)] py-[clamp(0.0975rem,0.3vw,0.125rem)] text-[0.58rem] font-semibold uppercase tracking-[0.12em] text-[var(--filelist-btn-text)] sm:px-[clamp(0.4rem,1vw,0.5rem)] sm:text-[0.68rem]">
                  {selectedCount} selected
                </span>
              )}
            </div>

            <div
              data-testid="trash-console-actions"
              className={`trashConsoleActions ${selectedCount > 0 ? "trashConsoleActionsSelected" : ""} flex w-full min-w-0 flex-col items-stretch gap-[clamp(0.5rem,1.25vw,0.75rem)] sm:w-auto sm:flex-row sm:flex-nowrap sm:items-center sm:justify-end sm:gap-[clamp(0.3rem,0.8vw,0.375rem)] sm:overflow-x-auto`}
            >
              <div
                data-testid="trash-batch-actions-row"
                className="trashBatchActionsRow flex w-full justify-start gap-[clamp(0.2rem,0.7vw,0.25rem)] sm:w-auto sm:justify-end sm:gap-[clamp(0.3rem,0.8vw,0.375rem)]"
              >
                <button
                  type="button"
                  disabled={files.length === 0}
                  aria-label={allVisibleSelected ? "取消全选" : "全选"}
                  aria-pressed={allVisibleSelected}
                  onClick={toggleAllSelected}
                  className="glass-btn toolbarActionBtn trashConsoleButton trashSelectAllButton allFilesBtnHighlight font-brand inline-flex w-auto max-w-max shrink-0 items-center justify-center gap-[clamp(0.2rem,0.7vw,0.25rem)] px-[clamp(0.3rem,0.8vw,0.375rem)] text-[0.6rem] font-normal leading-none tracking-widest text-[var(--filelist-btn-text)] transition-all hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-45 sm:gap-[clamp(0.3rem,0.8vw,0.375rem)] sm:px-[clamp(0.4rem,1vw,0.5rem)] sm:text-[0.68rem]"
                >
                  <CheckSquare className="h-[clamp(0.65rem,1.5vw,0.75rem)] w-[clamp(0.65rem,1.5vw,0.75rem)] sm:h-[clamp(0.75rem,1.8vw,0.875rem)] sm:w-[clamp(0.75rem,1.8vw,0.875rem)]" aria-hidden />
                  <span>{allVisibleSelected ? "取消全选" : "全选"}</span>
                </button>
                <button
                  type="button"
                  onClick={() => batchRestoreMutation.mutate(visibleSelectedIds)}
                  disabled={selectedCount === 0 || batchRestoreMutation.isPending}
                  className="glass-btn toolbarActionBtn trashConsoleButton trashBatchRestoreButton allFilesBtnHighlight font-brand inline-flex w-auto max-w-max shrink-0 items-center justify-center gap-[clamp(0.2rem,0.7vw,0.25rem)] px-[clamp(0.3rem,0.8vw,0.375rem)] text-[0.6rem] font-normal leading-none tracking-widest text-[var(--filelist-btn-text)] transition-all hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-45 sm:gap-[clamp(0.3rem,0.8vw,0.375rem)] sm:px-[clamp(0.4rem,1vw,0.5rem)] sm:text-[0.68rem]"
                  aria-label="批量还原"
                >
                  <ArchiveRestore className="h-[clamp(0.65rem,1.5vw,0.75rem)] w-[clamp(0.65rem,1.5vw,0.75rem)] sm:h-[clamp(0.75rem,1.8vw,0.875rem)] sm:w-[clamp(0.75rem,1.8vw,0.875rem)]" aria-hidden />
                  <span>批量还原</span>
                </button>
                <button
                  type="button"
                  onClick={() => setConfirm({ type: "batch-permanent" })}
                  disabled={selectedCount === 0 || batchPermanentMutation.isPending}
                  className="glass-btn toolbarActionBtn trashConsoleButton trashBatchPermanentButton uploadBtnHighlight font-brand inline-flex w-auto max-w-max shrink-0 items-center justify-center gap-[clamp(0.2rem,0.7vw,0.25rem)] px-[clamp(0.3rem,0.8vw,0.375rem)] text-[0.6rem] font-normal leading-none tracking-widest text-[var(--filelist-btn-text)] transition-all hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-45 sm:gap-[clamp(0.3rem,0.8vw,0.375rem)] sm:px-[clamp(0.4rem,1vw,0.5rem)] sm:text-[0.68rem]"
                  aria-label="批量彻底删除"
                >
                  <XCircle className="h-[clamp(0.65rem,1.5vw,0.75rem)] w-[clamp(0.65rem,1.5vw,0.75rem)] sm:h-[clamp(0.75rem,1.8vw,0.875rem)] sm:w-[clamp(0.75rem,1.8vw,0.875rem)]" aria-hidden />
                  <span>批量彻底删除</span>
                </button>
              </div>
              <div
                data-testid="trash-base-actions-row"
                className="trashBaseActionsRow ml-auto flex w-full justify-end gap-[clamp(0.2rem,0.7vw,0.25rem)] sm:w-auto sm:gap-[clamp(0.3rem,0.8vw,0.375rem)]"
              >
                <button
                  type="button"
                  onClick={handleBack}
                  className="glass-btn toolbarActionBtn trashConsoleButton trashBackButton allFilesBtnHighlight font-brand inline-flex w-auto max-w-max shrink-0 items-center justify-center gap-[clamp(0.2rem,0.7vw,0.25rem)] px-[clamp(0.3rem,0.8vw,0.375rem)] text-[0.6rem] font-normal leading-none tracking-widest text-[var(--filelist-btn-text)] transition-all hover:brightness-110 sm:gap-[clamp(0.3rem,0.8vw,0.375rem)] sm:px-[clamp(0.4rem,1vw,0.5rem)] sm:text-[0.68rem]"
                  aria-label="返回上一级"
                >
                  <ArrowLeft className="h-[clamp(0.65rem,1.5vw,0.75rem)] w-[clamp(0.65rem,1.5vw,0.75rem)] sm:h-[clamp(0.75rem,1.8vw,0.875rem)] sm:w-[clamp(0.75rem,1.8vw,0.875rem)]" aria-hidden />
                  <span>返回上一级</span>
                </button>
                <button
                  type="button"
                  disabled={files.length === 0}
                  onClick={() => setConfirm({ type: "empty" })}
                  aria-label="清空回收站"
                  className="glass-btn toolbarActionBtn trashConsoleButton trashEmptyButton uploadBtnHighlight font-brand inline-flex w-auto max-w-max shrink-0 items-center justify-center gap-[clamp(0.2rem,0.7vw,0.25rem)] px-[clamp(0.3rem,0.8vw,0.375rem)] text-[0.6rem] font-normal leading-none tracking-widest text-[var(--filelist-btn-text)] transition-all hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-45 sm:gap-[clamp(0.3rem,0.8vw,0.375rem)] sm:px-[clamp(0.4rem,1vw,0.5rem)] sm:text-[0.68rem]"
                >
                  <Trash2 className="h-[clamp(0.65rem,1.5vw,0.75rem)] w-[clamp(0.65rem,1.5vw,0.75rem)] sm:h-[clamp(0.75rem,1.8vw,0.875rem)] sm:w-[clamp(0.75rem,1.8vw,0.875rem)]" aria-hidden />
                  <span>清空回收站</span>
                </button>
              </div>
            </div>
          </div>
        </div>

        {error && (
          <ErrorMessage
            message={error}
            type="error"
            onClose={() => setError(null)}
            autoDismissMs={5000}
          />
        )}

        {isLoading ? (
          <div className="flex min-h-[clamp(10rem,24vw,12rem)] items-center justify-center rounded-[clamp(0.4rem,1vw,0.5rem)] border border-[var(--color-border-soft)] bg-[rgba(var(--rgb-white),0.035)]">
            <Spinner size="lg" />
          </div>
        ) : files.length === 0 ? (
          <EmptyState
            title="回收站为空"
            description={`删除的文件会在这里保留 ${TRASH_RETENTION_DAYS} 天`}
            icon={<Trash2 className="h-[clamp(2rem,4.5vw,2.25rem)] w-[clamp(2rem,4.5vw,2.25rem)] text-[var(--color-text-muted)]" aria-hidden />}
          />
        ) : (
          <div className="space-y-[clamp(0.75rem,2vw,1rem)]">
            <div
              data-testid="trash-card-grid"
              className="grid grid-cols-3 gap-x-[clamp(0.4rem,1vw,0.5rem)] gap-y-[clamp(0.6rem,1.4vw,0.75rem)] sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-8 xl:grid-cols-10"
            >
              {files.map((file) => {
                const retention = getRetentionState(file.deleted_at, retentionNow);
                const isSelected = selectedIdSet.has(file.id);
                const mimeTypeLabel = getMimeTypeLabel(
                  file.mime_type,
                  file.original_filename,
                );

                return (
                  // The whole card toggles selection; the checkbox gives a visible target.
                  <article
                    key={file.id}
                    data-testid={`trash-card-${file.id}`}
                    aria-label={`${file.original_filename} trash card`}
                    aria-selected={isSelected}
                    onClick={() => toggleSelected(file.id)}
                    className={`glass-card trashCardFrame group relative cursor-pointer rounded-[clamp(0.3rem,0.8vw,0.375rem)] transition-colors ${isSelected ? "trashCardSelected border-[var(--cta-primary-border)]" : ""}`}
                  >
                    <div className="p-[clamp(0.6rem,1.4vw,0.75rem)]">
                      <div
                        data-testid={`trash-card-thumbnail-${file.id}`}
                        className="trashCardThumb relative mb-[clamp(0.6rem,1.4vw,0.75rem)] aspect-square cursor-pointer overflow-hidden rounded-[clamp(0.2rem,0.6vw,0.25rem)] bg-[var(--file-card-thumb-bg)]"
                      >
                        <SelectionCheckbox
                          isSelected={isSelected}
                          onClick={() => toggleSelected(file.id)}
                          size="responsive"
                          positionClassName="absolute left-[clamp(0.15rem,0.35vw,0.25rem)] top-[clamp(0.15rem,0.35vw,0.25rem)]"
                        />

                        <TrashThumbnail file={file} />
                      </div>

                      <div
                        data-testid={`trash-card-meta-${file.id}`}
                        className="trashCardMeta relative flex w-full flex-col items-center gap-[clamp(0.18rem,0.55vw,0.32rem)] text-center"
                      >
                        <p
                          data-testid={`trash-card-title-${file.id}`}
                          className="min-w-0 w-full truncate whitespace-nowrap pr-0 text-center text-[clamp(0.38rem,1.3vw,0.58rem)] font-medium leading-[1.3] text-[var(--file-card-text)]"
                          title={file.original_filename}
                        >
                          {file.original_filename}
                        </p>
                        <p
                          data-testid={`trash-card-detail-${file.id}`}
                          className="flex min-w-0 w-full items-center justify-center gap-[clamp(0.2rem,0.7vw,0.25rem)] overflow-hidden whitespace-nowrap text-center text-[clamp(0.38rem,1.25vw,0.55rem)] text-[var(--file-card-text-muted)]"
                        >
                          <span className="shrink-0">
                            {formatFileSizeCompact(file.file_size)}
                          </span>
                          <span
                            className="h-[clamp(0.1rem,0.3vw,0.125rem)] w-[clamp(0.1rem,0.3vw,0.125rem)] rounded-full bg-[var(--color-border-medium)]"
                            aria-hidden="true"
                          />
                          <span className="min-w-0 truncate">{mimeTypeLabel}</span>
                        </p>
                        <p
                          data-testid={`trash-card-footer-countdown-${file.id}`}
                          className={`min-w-0 w-full truncate whitespace-nowrap text-center text-[clamp(0.38rem,1.25vw,0.55rem)] ${getCountdownClass(retention.daysLeft)}`}
                        >
                          {retention.daysLeft} days left
                        </p>
                        <div
                          data-testid={`trash-card-actions-${file.id}`}
                          className="trashCardActions flex w-full items-center justify-center gap-[clamp(0.18rem,0.55vw,0.28rem)] pt-[clamp(0.15rem,0.45vw,0.25rem)]"
                        >
                          <button
                            type="button"
                            onClick={(event) => {
                              event.stopPropagation();
                              restoreMutation.mutate(file.id);
                            }}
                            disabled={restoreMutation.isPending}
                            data-testid={`trash-card-restore-${file.id}`}
                            className="glass-btn trashCardActionButton allFilesBtnHighlight inline-flex min-w-0 flex-1 items-center justify-center p-0 text-[var(--filelist-btn-text)] transition hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-60"
                            aria-label={`还原 ${file.original_filename}`}
                            title="Restore"
                          >
                            <ArchiveRestore className="trashCardActionIcon" aria-hidden />
                          </button>
                          <button
                            type="button"
                            onClick={(event) => {
                              event.stopPropagation();
                              setConfirm({ type: "permanent", file });
                            }}
                            disabled={permanentMutation.isPending}
                            className="glass-btn trashCardActionButton uploadBtnHighlight inline-flex min-w-0 flex-1 items-center justify-center p-0 text-[var(--filelist-btn-text)] transition hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-60"
                            aria-label={`彻底删除 ${file.original_filename}`}
                            title="Purge"
                          >
                            <XCircle className="trashCardActionIcon" aria-hidden />
                          </button>
                        </div>
                      </div>
                    </div>
                  </article>
                );
              })}
            </div>
          </div>
        )}
      </div>

      <ConfirmDialog
        open={confirm !== null}
        title={confirmTitle}
        message={confirmMessage}
        confirmText={confirmText}
        variant="danger"
        appearance="glass"
        loading={confirmLoading}
        onCancel={() => setConfirm(null)}
        onConfirm={() => {
          if (confirm?.type === "empty") {
            emptyMutation.mutate();
          } else if (confirm?.type === "batch-permanent") {
            batchPermanentMutation.mutate(visibleSelectedIds);
          } else if (confirm?.type === "permanent") {
            permanentMutation.mutate(confirm.file.id);
          }
        }}
      />
    </PageLayout>
  );
}
