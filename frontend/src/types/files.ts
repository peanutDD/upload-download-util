/**
 * 文件相关类型定义
 */

/**
 * 文件元数据类型
 */
export interface FileMetadata {
  id: string;
  filename: string;
  original_filename: string;
  file_size: number;
  mime_type: string;
  category: string | null;
  folder_id: string | null;
  created_at: string;
  deleted_at?: string | null;
  search_snippet?: string;
  match_source?: 'filename' | 'content' | 'ocr' | 'category';
  search_score?: number;
}

/**
 * 文件列表响应类型（支持传统分页和游标分页）
 */
export interface FileListResponse {
  files: FileMetadata[];
  // 传统分页字段（当使用 page 参数时）
  total?: number;
  page?: number;
  limit?: number;
  // 游标分页字段（当使用 cursor 参数时）
  next_cursor?: string | null;
}

export interface TrashListResponse {
  files: FileMetadata[];
}

/**
 * 文件列表查询参数类型
 */
export interface FileListQuery {
  // 传统分页参数
  page?: number;
  limit?: number;
  // 游标分页参数（如果提供了 cursor，则使用游标分页，忽略 page）
  cursor?: string;
  // 筛选参数
  search?: string;
  mime_type?: string;
  category?: string;
  folder_id?: string | null;
  date_from?: string;
  date_to?: string;
  size_min?: number;
  size_max?: number;
  // 排序参数
  sort_by?: 'created_at' | 'filename' | 'file_size' | 'type';
  sort_order?: 'asc' | 'desc';
}

/**
 * 上传队列项类型
 */
export interface QueueItem<T = unknown> {
  id: string;
  task: () => Promise<T>;
  resolve: (value: T) => void;
  reject: (reason?: unknown) => void;
  priority: number;
  cost: number;
}

/**
 * 缓存的文件列表类型
 */
export interface CachedFileList {
  version: number;
  files: FileMetadata[];
  total: number;
  timestamp: number;
  queryKey: string;
}

/**
 * 分块上传初始化响应类型
 */
export interface ChunkedUploadInitResponse {
  upload_id: string;
  chunk_size: number;
  total_parts: number;
}

/**
 * 分块上传状态响应类型
 */
export interface ChunkedUploadStatusResponse {
  uploaded_parts: number[];
  total_parts: number;
}

/**
 * 批量操作响应类型
 */
export interface BatchOperationResponse {
  deleted?: number;
  moved?: number;
  message?: string;
}

/**
 * 存储使用情况类型
 */
export interface StorageUsage {
  total_size: number;
  file_count: number;
  total_size_mb: number;
  quota: number | null;
  quota_mb: number | null;
  usage_percent: number | null;
  is_unlimited: boolean;
}

/**
 * 分类类型
 */
export interface Category {
  id: string;
  name: string;
  created_at: string;
}
