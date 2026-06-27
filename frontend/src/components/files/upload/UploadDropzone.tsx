import { useRef, useCallback, useEffect } from "react";
import { Files, UploadCloud } from "lucide-react";
import { cn } from "../../../utils/cn";
import { getMaxFileSizeGB } from "../../../utils/uploadValidation";

interface UploadDropzoneProps {
  /** 拖拽状态 */
  dragActive: boolean;
  /** 拖拽事件处理 */
  onDragEnter: (e: React.DragEvent) => void;
  onDragLeave: (e: React.DragEvent) => void;
  onDragOver: (e: React.DragEvent) => void;
  onDrop: (e: React.DragEvent) => void;
  /** 文件选择回调 */
  onFilesSelect: (files: File[]) => void;
  /** 对话框是否打开（用于重置 input） */
  open?: boolean;
}

/**
 * 文件拖拽区域组件
 */
export default function UploadDropzone({
  dragActive,
  onDragEnter,
  onDragLeave,
  onDragOver,
  onDrop,
  onFilesSelect,
  open,
}: UploadDropzoneProps) {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const maxGB = getMaxFileSizeGB();

  // 打开弹窗时清空 input
  useEffect(() => {
    if (open && inputRef.current) {
      inputRef.current.value = "";
      inputRef.current.setAttribute("multiple", "");
    }
  }, [open]);

  const handleFileChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const list = e.target.files;
      if (!list || list.length === 0) return;
      onFilesSelect(Array.from(list));
      // 清空 input 以允许重复选择相同文件
      setTimeout(() => {
        if (inputRef.current) inputRef.current.value = "";
      }, 0);
    },
    [onFilesSelect],
  );

  const handleSelectClick = useCallback(() => {
    if (inputRef.current) {
      inputRef.current.value = "";
      inputRef.current.click();
    }
  }, []);

  return (
    <div
      onDragEnter={onDragEnter}
      onDragLeave={onDragLeave}
      onDragOver={onDragOver}
      onDrop={onDrop}
      className={cn(
        "uploadDialogCyberDropzone relative mb-[clamp(0.78rem,1.8vw,1rem)] flex flex-col items-center justify-center overflow-hidden rounded-[clamp(0.6rem,1.4vw,0.75rem)] border-2 border-dashed px-[clamp(1.25rem,2.7vw,1.5rem)] py-[clamp(1rem,2.25vw,1.25rem)] text-center transition-colors duration-200 sm:py-[clamp(1.25rem,2.7vw,1.5rem)]",
        dragActive
          ? "uploadDialogCyberDropzoneActive border-[var(--upload-accent)] bg-[var(--upload-accent-bg)]"
          : "border-[var(--upload-drop-border)] bg-[var(--upload-drop-bg)] hover:border-[var(--upload-drop-border-hover)]",
      )}
      data-oid="_z50by4"
    >
      <input
        ref={inputRef}
        type="file"
        multiple
        className="hidden"
        aria-label="选择文件"
        onChange={handleFileChange}
        data-oid="cmzy5dv"
      />

      <div className="uploadDropzoneCore mb-[clamp(0.585rem,1.35vw,0.75rem)] flex h-[clamp(3.25rem,6.3vw,3.5rem)] w-[clamp(3.25rem,6.3vw,3.5rem)] items-center justify-center rounded-[clamp(0.6rem,1.4vw,0.75rem)]" data-oid="ybxiui-">
        <Files className="uploadDropzoneCoreBack h-[clamp(1.75rem,3.6vw,2rem)] w-[clamp(1.75rem,3.6vw,2rem)]" aria-hidden="true" />
        <UploadCloud className="uploadDropzoneCoreFront h-[clamp(1.75rem,3.6vw,2rem)] w-[clamp(1.75rem,3.6vw,2rem)]" aria-hidden="true" />
      </div>

      <p
        className="font-brand mb-[clamp(0.195rem,0.45vw,0.25rem)] text-[clamp(0.75rem,1.8vw,0.875rem)] font-normal tracking-widest text-[var(--upload-text)]"
        data-oid="rg8qpyg"
      >
        Drag and drop your files
      </p>
      <p
        className="font-brand mb-[clamp(0.78rem,1.8vw,1rem)] text-[clamp(0.68rem,1.6vw,0.75rem)] font-normal tracking-widest text-[var(--upload-text-muted)]"
        data-oid="-vtfj4r"
      >
        Max. File size:{" "}
        {maxGB > 1 ? `${maxGB} GB` : `${Math.round(maxGB * 1024)} MB`}
      </p>

      <button
        type="button"
        onClick={handleSelectClick}
        className="uploadDialogCyberPrimaryBtn uploadDialogSelectFilesBtn font-brand inline-flex items-center justify-center gap-[clamp(0.39rem,0.9vw,0.5rem)] rounded-[clamp(0.4rem,1vw,0.5rem)] bg-[var(--btn-primary-bg)] px-[clamp(1rem,2.25vw,1.25rem)] py-[clamp(0.4875rem,1.125vw,0.625rem)] text-[clamp(0.75rem,1.8vw,0.875rem)] font-normal tracking-widest text-[var(--btn-primary-text)] transition-colors hover:bg-[var(--btn-primary-bg-hover)]"
        data-oid="xdol0-4"
      >
        <UploadCloud className="h-[clamp(0.78rem,1.8vw,1rem)] w-[clamp(0.78rem,1.8vw,1rem)]" aria-hidden="true" />
        Select files
      </button>
      <p
        className="font-brand mt-[clamp(0.39rem,0.9vw,0.5rem)] text-[clamp(0.68rem,1.6vw,0.75rem)] text-[var(--upload-text-muted)]"
        data-oid="d5m9iy1"
      >
        支持多选；若多选只显示 1 个，请将多个文件
        <strong data-oid="0fhzygb">拖入上方区域</strong>
        ，或多次点击「Select files」逐个添加
      </p>
    </div>
  );
}
