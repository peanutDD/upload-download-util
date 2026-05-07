/**
 * App-wide constants. Avoid magic numbers and duplicated config.
 */

/** 类型筛选项「仅文件夹」的占位值，不传给后端 mime_type */
export const MIME_FILTER_FOLDERS = '__folders__' as const;

export const FILE_LIST = {
  ROW_HEIGHT: 72,
  LIST_HEIGHT: 480,
  /** 网格模式下每行估算高度（用于虚拟列表 fallback；实际运行会按容器宽度动态估算） */
  VIRTUAL_GRID_ROW_HEIGHT: 320,
  // 单页文件数量：减小以降低首屏渲染压力
  LIMIT: 30,
  CACHE_MINUTES: 5,
  /** 超过该数量时启用虚拟列表 */
  VIRTUAL_THRESHOLD: 24,
  /** 缓存版本号：数据结构变更时递增，旧缓存自动失效 */
  CACHE_VERSION: 1,
  /** 文件列表缓存最大条数（LRU 淘汰，避免 localStorage 溢出） */
  CACHE_MAX_ENTRIES: 80,
} as const;

/**
 * 批量操作限制（避免一次性选择/下载导致卡顿、超时、内存暴涨）
 *
 * 说明：
 * - 前端做“软限制”（提前提示用户），后端仍会做“硬限制”（安全兜底）。
 */
export const BATCH_LIMITS = {
  /** 单次批量下载 ZIP 最多包含的文件数（含从文件夹递归展开出来的文件） */
  MAX_DOWNLOAD_ZIP_FILES: 200,
} as const;

/**
 * 分块上传配置
 * 使用现代上传技术：分块、并行、断点续传
 */
export const CHUNKED_UPLOAD = {
  // 分块大小：2MB（弱网/移动网络更稳）
  CHUNK_SIZE: 2 * 1024 * 1024,
  // 触发分块上传的阈值：2MB 以上使用分块
  THRESHOLD: 2 * 1024 * 1024,
  // 最大重试次数
  MAX_RETRIES: 5,
  // 并行上传的块数（bounded worker pool，分片完成后立即补位）
  PARALLEL_CHUNKS: 4,
  // 重试延迟（毫秒），指数退避
  RETRY_DELAY_BASE: 1000,
} as const;

/**
 * 大文件（分片上传）数量限制，与后端约定一致
 * 超过 100MB 视为大文件，最多同时 10 个在队列中
 */
export const LARGE_FILE_UPLOAD = {
  /** 视为大文件的体积阈值（字节），≥ 此值走分片上传且计入大文件数量 */
  SIZE_THRESHOLD_BYTES: 100 * 1024 * 1024,
  /** 每用户同时进行的大文件（分片）上传数量上限 */
  MAX_CONCURRENT: 10,
} as const;

/**
 * 上传队列：按成本限制并发（大文件占更多槽位），支持优先级
 */
export const UPLOAD_QUEUE = {
  /** 总成本上限：小文件 cost=1，大文件 cost=3。设为 10 可同时跑 3 大(3*3=9)+1 小(1)，大文件上传时小文件不阻塞 */
  MAX_COST: 10,
  /** 大于等于此大小视为大文件，占用 3 成本；否则 1 成本 */
  LARGE_FILE_THRESHOLD_BYTES: 10 * 1024 * 1024,
} as const;

export const SIZES = ['Bytes', 'KB', 'MB', 'GB', 'TB'] as const;

/**
 * 前端降低后端并发压力相关常量
 */
export const REQUEST = {
  /** 全局请求并发上限 */
  LIMITER_MAX_CONCURRENT: 20,
  /** 批量请求收集窗口（ms） */
  BATCH_DELAY_MS: 50,
  /** GET 请求重试次数 */
  RETRY_MAX: 3,
  /** GET 请求重试初始延迟（ms） */
  RETRY_INITIAL_DELAY_MS: 1000,
  /** GET 请求重试最大延迟（ms） */
  RETRY_MAX_DELAY_MS: 10000,
  /** 全局请求去重：相同请求在此时间（ms）内复用结果，不重复发请求 */
  DEDUP_TTL_MS: 5000,
  /** 全局请求去重：响应缓存最大条数 */
  DEDUP_MAX_CACHE_SIZE: 100,
} as const;
