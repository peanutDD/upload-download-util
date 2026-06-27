import { useEffect, useRef, useState, useCallback } from "react";
import { Link2, SatelliteDish } from "lucide-react";
import { cn } from "../../../utils/cn";
import {
  getMaxFileSizeBytes,
  getUploadMimeType,
  validateFile,
} from "../../../utils/uploadValidation";
import type { UploadFile } from "./UploadFileItem";

const REMOTE_URL_HEAD_TIMEOUT_MS = 30_000;

interface UrlUploadFormProps {
  /** 添加文件到上传列表 */
  onFileAdd: (file: UploadFile) => void;
}

/**
 * 获取 URL 上传错误消息
 */
function getUrlErrorMessage(err: unknown, url: string): string {
  if (err instanceof TypeError && err.message.includes("URL")) {
    return `URL 格式无效: "${url}"。请输入完整的 URL，例如 https://example.com/file.jpg`;
  }

  if (err instanceof TypeError) {
    if (
      err.message.includes("Failed to fetch") ||
      err.message.includes("NetworkError")
    ) {
      return `无法访问该 URL。可能的原因：
• 目标服务器不允许跨域请求 (CORS)
• URL 地址不存在或无法访问
• 网络连接问题`;
    }
    return `网络请求失败: ${err.message}`;
  }

  if (err instanceof Error) {
    const httpMatch = err.message.match(/^HTTP (\d+)(?:\s*-\s*(.+))?$/);
    if (httpMatch) {
      const status = parseInt(httpMatch[1], 10);
      const statusMessages: Record<number, string> = {
        400: "请求无效",
        401: "需要身份验证",
        403: "访问被拒绝（无权限）",
        404: "文件不存在",
        405: "请求方法不允许",
        408: "请求超时",
        410: "资源已被删除",
        429: "请求过于频繁",
        500: "服务器内部错误",
        502: "网关错误",
        503: "服务暂时不可用",
        504: "网关超时",
      };
      const statusText = statusMessages[status] || "请求失败";
      return `下载失败 (HTTP ${status}): ${statusText}`;
    }
    return err.message;
  }

  return "URL 下载失败，请检查地址是否正确";
}

function getFilenameFromContentDisposition(header: string | null): string | null {
  if (!header) return null;
  const utf8Match = header.match(/filename\*=UTF-8''([^;]+)/i);
  if (utf8Match) {
    try {
      return decodeURIComponent(utf8Match[1].trim().replace(/^"|"$/g, ""));
    } catch {
      return null;
    }
  }

  const plainMatch = header.match(/filename="?([^";]+)"?/i);
  return plainMatch ? plainMatch[1].trim() : null;
}

function getFilenameFromUrl(urlObj: URL): string {
  const pathname = urlObj.pathname;
  const rawFilename = pathname.split("/").pop() || "downloaded-file";
  try {
    return decodeURIComponent(rawFilename);
  } catch {
    return rawFilename;
  }
}

function assertRemoteFileSizeAllowed(response: Response): void {
  const contentLength = response.headers.get("content-length");
  if (!contentLength) return;

  const size = Number(contentLength);
  if (!Number.isFinite(size) || size <= 0) return;

  const maxBytes = getMaxFileSizeBytes();
  if (size > maxBytes) {
    const maxGB = (maxBytes / (1024 * 1024 * 1024)).toFixed(1);
    throw new Error(`远程文件超过 ${maxGB}GB 限制`);
  }
}

/**
 * URL 上传表单组件
 */
export default function UrlUploadForm({ onFileAdd }: UrlUploadFormProps) {
  const [urlInput, setUrlInput] = useState("");
  const [loading, setLoading] = useState(false);
  const activeControllerRef = useRef<AbortController | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      activeControllerRef.current?.abort();
    };
  }, []);

  const handleUpload = useCallback(async () => {
    const trimmedUrl = urlInput.trim();
    if (!trimmedUrl) return;

    setLoading(true);
    try {
      // 验证 URL 格式
      let urlObj: URL;
      try {
        urlObj = new URL(trimmedUrl);
      } catch {
        throw new TypeError(`Invalid URL: ${trimmedUrl}`);
      }

      // 验证协议
      if (!["http:", "https:"].includes(urlObj.protocol)) {
        throw new Error(
          `不支持的协议: ${urlObj.protocol}。仅支持 http:// 和 https://`,
        );
      }

      const fallbackFilename = getFilenameFromUrl(urlObj);

      // 使用 AbortController 设置超时
      const controller = new AbortController();
      activeControllerRef.current = controller;
      const timeoutId = setTimeout(
        () => controller.abort(),
        REMOTE_URL_HEAD_TIMEOUT_MS,
      );

      let response: Response;
      try {
        response = await fetch(trimmedUrl, {
          signal: controller.signal,
          mode: "cors",
        });
      } catch (fetchErr) {
        clearTimeout(timeoutId);
        if (fetchErr instanceof Error && fetchErr.name === "AbortError") {
          throw new Error("HTTP 408 - 请求超时（超过30秒）");
        }
        throw fetchErr;
      }
      clearTimeout(timeoutId);

      if (!response.ok) {
        throw new Error(`HTTP ${response.status} - ${response.statusText}`);
      }
      assertRemoteFileSizeAllowed(response);

      const blob = await response.blob();

      if (blob.size === 0) {
        throw new Error("下载的文件为空");
      }

      const filename =
        getFilenameFromContentDisposition(response.headers.get("content-disposition")) ??
        fallbackFilename;
      const file = new File([blob], filename, { type: blob.type });
      const validation = validateFile(file);

      const uploadFile: UploadFile = {
        id: `url-${Date.now()}-${Math.random().toString(36).slice(2)}`,
        name: filename,
        size: file.size,
        mimeType: getUploadMimeType(file),
        status: validation.ok ? "pending" : "error",
        progress: 0,
        error: validation.ok ? undefined : validation.error,
        file: validation.ok ? file : undefined,
      };

      onFileAdd(uploadFile);
      setUrlInput("");
    } catch (err) {
      if (!mountedRef.current) return;
      const errorFile: UploadFile = {
        id: `url-error-${Date.now()}`,
        name:
          trimmedUrl.length > 50 ? trimmedUrl.slice(0, 50) + "..." : trimmedUrl,
        size: 0,
        mimeType: "unknown",
        status: "error",
        progress: 0,
        error: getUrlErrorMessage(err, trimmedUrl),
      };
      onFileAdd(errorFile);
    } finally {
      activeControllerRef.current = null;
      if (mountedRef.current) setLoading(false);
    }
  }, [urlInput, onFileAdd]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") handleUpload();
    },
    [handleUpload],
  );

  return (
    <div className="uploadUrlPanel mb-[clamp(0.78rem,1.8vw,1rem)]" data-oid="_b1-ohy">
      <p
        className="font-brand mb-[clamp(0.39rem,0.9vw,0.5rem)] flex items-center gap-[clamp(0.39rem,0.9vw,0.5rem)] text-[clamp(0.75rem,1.8vw,0.875rem)] font-normal tracking-widest text-[var(--upload-text-muted)]"
        data-oid="mcl5tfe"
      >
        <SatelliteDish className="h-[clamp(0.78rem,1.8vw,1rem)] w-[clamp(0.78rem,1.8vw,1rem)] text-[var(--upload-accent)]" aria-hidden="true" />
        Or upload from URL
      </p>
      <div className="flex gap-[clamp(0.39rem,0.9vw,0.5rem)]" data-oid="h__ldbv">
        <label htmlFor="upload-url-input" className="sr-only">
          File URL
        </label>
        <input
          id="upload-url-input"
          type="url"
          name="uploadUrl"
          autoComplete="off"
          aria-label="File URL"
          value={urlInput}
          onChange={(e) => setUrlInput(e.target.value)}
          placeholder="Add file URL"
          className={cn(
            "uploadDialogCyberInput font-brand flex-1 rounded-[clamp(0.4rem,1vw,0.5rem)] border bg-transparent px-[clamp(0.78rem,1.8vw,1rem)] py-[clamp(0.4875rem,1.125vw,0.625rem)] text-[clamp(0.75rem,1.8vw,0.875rem)] font-normal tracking-widest text-[var(--upload-input-text)] placeholder-[var(--upload-input-placeholder)] transition-colors focus:outline-none",
            urlInput.trim()
              ? "uploadDialogCyberInputActive border-[var(--upload-accent)]"
              : "border-[var(--upload-input-border)] focus:border-[var(--upload-accent)]",
          )}
          onKeyDown={handleKeyDown}
          data-oid="shiu-r."
        />

        <button
          type="button"
          onClick={handleUpload}
          disabled={!urlInput.trim() || loading}
          className="uploadDialogCyberPrimaryBtn uploadDialogUrlUploadBtn font-brand inline-flex items-center justify-center gap-[clamp(0.39rem,0.9vw,0.5rem)] rounded-[clamp(0.4rem,1vw,0.5rem)] bg-[var(--btn-primary-bg)] px-[clamp(1rem,2.25vw,1.25rem)] py-[clamp(0.4875rem,1.125vw,0.625rem)] text-[clamp(0.75rem,1.8vw,0.875rem)] font-normal tracking-widest text-[var(--btn-primary-text)] transition-colors hover:bg-[var(--btn-primary-bg-hover)] disabled:cursor-not-allowed disabled:opacity-50"
          data-oid="dp9.vve"
        >
          <Link2 className="h-[clamp(0.78rem,1.8vw,1rem)] w-[clamp(0.78rem,1.8vw,1rem)]" aria-hidden="true" />
          {loading ? "…" : "Upload"}
        </button>
      </div>
    </div>
  );
}
