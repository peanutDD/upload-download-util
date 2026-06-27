/**
 * MIME 类型工具函数
 *
 * 提供统一的 MIME 类型判断和标签生成函数，避免跨组件的代码重复。
 */

// ============================================================================
// 类型判断函数
// ============================================================================

/** 判断是否为图片类型 */
export const isImageType = (mime: string): boolean => mime.startsWith('image/');

/** 判断是否为 GIF 图片（部分场景下按“视频”处理以获得更好的预览体验） */
export const isGifType = (mime: string): boolean =>
  mime.toLowerCase().startsWith('image/gif');

/** 判断是否为视频类型 */
export const isVideoType = (mime: string): boolean => mime.startsWith('video/');

/** 判断是否为 PDF 类型 */
export const isPdfType = (mime: string): boolean => mime === 'application/pdf';

/** 判断是否为音频类型 */
export const isAudioType = (mime: string): boolean => mime.startsWith('audio/');

/** 判断是否为文本类型（包括代码文件） */
export const isTextType = (mime: string): boolean =>
  mime.startsWith('text/') ||
  mime === 'application/json' ||
  mime === 'application/xml' ||
  mime === 'application/javascript';

/** 判断是否为 Markdown（基于 MIME 与文件名） */
export const isMarkdownType = (mime: string, filename?: string): boolean => {
  const lowerMime = mime.toLowerCase();
  const lowerName = filename?.toLowerCase() ?? '';
  return (
    lowerMime === 'text/markdown' ||
    lowerMime === 'text/x-markdown' ||
    lowerName.endsWith('.md') ||
    lowerName.endsWith('.markdown')
  );
};

/** 判断是否为文档类型 */
export const isDocumentType = (mime: string): boolean =>
  mime.includes('word') || mime.includes('document');

/** 判断是否为表格类型 */
export const isSpreadsheetType = (mime: string): boolean =>
  mime.includes('excel') || mime.includes('spreadsheet');

/** 判断是否为压缩包类型 */
export const isArchiveType = (mime: string): boolean =>
  mime.includes('zip') || mime.includes('rar') || mime.includes('7z') || mime.includes('tar');

// ============================================================================
// 标签生成函数
// ============================================================================

/** MIME 类型分类信息 */
export interface MimeTypeInfo {
  /** 分类标签 */
  label: string;
  /** 图标颜色 */
  color: string;
  /** 背景色类名 */
  bgClass: string;
}

/** 获取 MIME 类型的分类标签 */
export function getMimeTypeLabel(mime: string, filename?: string): string {
  // 使用条件表达式链替代 if-else
  return isImageType(mime)
    ? mime.split('/')[1]?.toUpperCase() || 'Image'
    : isVideoType(mime)
      ? 'Video'
      : isAudioType(mime)
        ? 'Audio'
        : isPdfType(mime)
          ? 'PDF'
          : isDocumentType(mime)
            ? 'Document'
            : isSpreadsheetType(mime)
              ? 'Spreadsheet'
              : isArchiveType(mime)
                ? 'Archive'
                : isMarkdownType(mime, filename)
                  ? 'md'
                  : isTextType(mime)
                    ? mime === 'application/json'
                      ? 'JSON'
                      : 'Text'
                    : 'File';
}

/** 获取 MIME 类型的完整信息（标签、颜色、背景） */
export function getMimeTypeInfo(mime: string): MimeTypeInfo {
  // 使用 Map 查找替代多个 if-else
  const typeMap: Array<[() => boolean, MimeTypeInfo]> = [
    [() => isVideoType(mime), { label: 'VIDEO', color: 'var(--upload-mime-video-color)', bgClass: 'bg-[var(--upload-mime-video-bg)]' }],
    [() => isPdfType(mime), { label: 'PDF', color: 'var(--upload-mime-pdf-color)', bgClass: 'bg-[var(--upload-mime-pdf-bg)]' }],
    [() => isAudioType(mime), { label: 'AUDIO', color: 'var(--upload-mime-audio-color)', bgClass: 'bg-[var(--upload-mime-audio-bg)]' }],
    [() => isImageType(mime), { label: 'IMAGE', color: 'var(--upload-mime-image-color)', bgClass: 'bg-[var(--upload-mime-image-bg)]' }],
    [() => isDocumentType(mime), { label: 'DOC', color: 'var(--upload-mime-doc-color)', bgClass: 'bg-[var(--upload-mime-doc-bg)]' }],
    [() => isSpreadsheetType(mime), { label: 'SHEET', color: 'var(--upload-mime-sheet-color)', bgClass: 'bg-[var(--upload-mime-sheet-bg)]' }],
    [() => isArchiveType(mime), { label: 'ZIP', color: 'var(--upload-mime-archive-color)', bgClass: 'bg-[var(--upload-mime-archive-bg)]' }],
    // 文本等其他类型：占位背景与视频卡片保持一致的紫色系
    [() => isTextType(mime), { label: 'TEXT', color: 'var(--upload-mime-text-color)', bgClass: 'bg-[var(--upload-mime-text-bg)]' }],
  ];

  const match = typeMap.find(([predicate]) => predicate());
  return match?.[1] ?? { label: 'FILE', color: 'var(--upload-mime-default-color)', bgClass: 'bg-[var(--upload-mime-default-bg)]' };
}

/** 获取图标颜色（用于 UploadFileItem 等组件） */
export function getMimeTypeColor(mime: string): string {
  return getMimeTypeInfo(mime).color;
}

// ============================================================================
// 预览支持判断
// ============================================================================

/** 判断文件类型是否支持预览 */
export function isPreviewSupported(mime: string): boolean {
  return (
    isImageType(mime) ||
    isPdfType(mime) ||
    isTextType(mime) ||
    isVideoType(mime) ||
    isAudioType(mime)
  );
}

const PREVIEW_MIME_BY_EXTENSION: Record<string, string> = {
  csv: 'text/csv',
  gif: 'image/gif',
  jpeg: 'image/jpeg',
  jpg: 'image/jpeg',
  json: 'application/json',
  m4a: 'audio/mp4',
  markdown: 'text/markdown',
  md: 'text/markdown',
  mov: 'video/quicktime',
  mp3: 'audio/mpeg',
  mp4: 'video/mp4',
  pdf: 'application/pdf',
  png: 'image/png',
  txt: 'text/plain',
  wav: 'audio/wav',
  webm: 'video/webm',
  webp: 'image/webp',
  xml: 'application/xml',
};

function extensionFromFilename(filename?: string): string {
  if (!filename) return '';
  const lastDot = filename.lastIndexOf('.');
  if (lastDot < 0 || lastDot === filename.length - 1) return '';
  return filename.slice(lastDot + 1).toLowerCase();
}

function effectivePreviewMime(mime: string, filename?: string): string {
  const normalized = mime.trim().toLowerCase();
  if (normalized && normalized !== 'application/octet-stream') return normalized;
  return PREVIEW_MIME_BY_EXTENSION[extensionFromFilename(filename)] ?? normalized;
}

/** 获取预览类型信息 */
export function getPreviewKind(mime: string, filename?: string) {
  const effectiveMime = effectivePreviewMime(mime, filename);
  const isGif = isGifType(effectiveMime);
  const isMarkdown = isMarkdownType(effectiveMime, filename);

  return {
    supported: isPreviewSupported(effectiveMime) || isMarkdown,
    // GIF 在预览层面按“视频”处理（走 <video> 管线），避免大 GIF 卡顿
    isImage: isImageType(effectiveMime) && !isGif,
    isPDF: isPdfType(effectiveMime),
    isText: isTextType(effectiveMime),
    isMarkdown,
    isVideo: isVideoType(effectiveMime) || isGif,
    isAudio: isAudioType(effectiveMime),
    // Ugoira 支持已移除，这里固定为 false，避免前端再走任何 Ugoira 分支
    isUgoira: false,
  };
}
