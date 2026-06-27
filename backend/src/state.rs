//! # 应用状态模块
//!
//! 统一管理应用的共享状态，包括配置、数据库连接池、存储后端和缓存服务。
//!
//! 使用 `State<AppState>` 替代多个 `Extension` 注入，
//! 符合 Axum 0.7 最佳实践。

use std::sync::Arc;

use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;
use tokio::sync::Semaphore;

use crate::config::Config;
use crate::repositories::{
    CachedFilesRepo, DynFileVersionsRepo, DynFilesRepo, DynUsersRepo, SqlxFileVersionsRepo,
    SqlxFilesRepo, SqlxUsersRepo,
};
use crate::services::cache::{files::FileCacheService, CacheService};
use crate::services::embeddings::EmbeddingService;
use crate::services::file::FileService;
use crate::services::fulltext_search::SearchIndexService;
use crate::services::storage::StorageBackend;
use crate::services::task_queue::{DynTaskQueue, TaskQueue};

/// 应用共享状态
///
/// 包含所有 handlers 和 services 需要的共享依赖：
/// - `config`: 应用配置（JWT、存储、文件限制等）
/// - `pool`: PostgreSQL 数据库连接池
/// - `storage`: 文件存储后端（本地或 S3）
/// - `cache`: 内存缓存服务
/// - `file_service`: 文件服务（统一注入，handlers 直接使用）
///
/// # 使用示例
///
/// ```rust,ignore
/// async fn my_handler(
///     State(state): State<AppState>,
/// ) -> Result<Response, AppError> {
///     state.file_service.some_method(...).await
/// }
/// ```
#[derive(Clone)]
pub struct AppState {
    /// 应用配置
    pub config: Arc<Config>,
    /// 数据库连接池
    pub pool: PgPool,
    pub read_pool: PgPool,
    pub redis: Option<RedisPool>,
    /// 存储后端（本地文件系统或 S3）
    pub storage: Arc<dyn StorageBackend>,
    /// 内存缓存服务
    pub cache: CacheService,
    /// 文件服务（统一构造并注入，避免各 handler 内重复 from_state）
    pub file_service: Arc<FileService>,
    /// 后台任务队列（GIF 转码、缩略图重建等）
    pub task_queue: DynTaskQueue,
    /// Tantivy 全文搜索索引（应用启动时初始化，避免请求内重复打开目录）。
    pub search_index: Arc<SearchIndexService>,
    /// 嵌入服务（用于语义搜索）
    pub embedding_service: Option<Arc<EmbeddingService>>,
    pub zip_build_semaphore: Arc<Semaphore>,
}

impl AppState {
    /// 创建新的应用状态
    ///
    /// # 参数
    /// - `config`: 应用配置
    /// - `pool`: 数据库连接池
    /// - `storage`: 存储后端
    pub fn new(
        config: Arc<Config>,
        pool: PgPool,
        read_pool: PgPool,
        storage: Arc<dyn StorageBackend>,
        redis: Option<RedisPool>,
    ) -> Self {
        let inner_files_repo = Arc::new(SqlxFilesRepo::new_with_replica(
            pool.clone(),
            read_pool.clone(),
        ));

        // 如果 Redis 可用且缓存启用，使用带缓存的文件仓库
        let files_repo: DynFilesRepo = if let Some(ref redis_pool) = redis {
            if config.cache.enabled {
                let file_cache = Arc::new(FileCacheService::new(
                    redis_pool.clone(),
                    Arc::new(config.cache.clone()),
                ));
                Arc::new(CachedFilesRepo::new(inner_files_repo, file_cache))
            } else {
                inner_files_repo
            }
        } else {
            inner_files_repo
        };

        let file_versions_repo: DynFileVersionsRepo =
            Arc::new(SqlxFileVersionsRepo::new(pool.clone()));
        let users_repo: DynUsersRepo = Arc::new(SqlxUsersRepo::new(pool.clone()));

        // 如果配置了 Hugging Face API Token，创建嵌入服务
        let embedding_service = if config.search.huggingface_api_token.is_some() {
            Some(Arc::new(EmbeddingService::new(&config)))
        } else {
            None
        };

        let file_service = Arc::new(FileService::new(
            files_repo,
            file_versions_repo,
            users_repo,
            pool.clone(),
            storage.clone(),
            config.clone(),
            embedding_service.clone(),
        ));

        let task_queue: DynTaskQueue = Arc::new(TaskQueue::new(Arc::new(pool.clone())));
        let search_index = Arc::new(
            SearchIndexService::open_or_create(&config.search.fulltext_index_path).unwrap_or_else(
                |error| {
                    tracing::error!(
                        error = %error,
                        path = %config.search.fulltext_index_path,
                        "failed to open fulltext index, falling back to in-memory index"
                    );
                    SearchIndexService::open_in_memory()
                        .expect("fallback in-memory fulltext index should initialize")
                },
            ),
        );
        let zip_build_semaphore = Arc::new(Semaphore::new(config.tasks.zip_build_max_concurrent));

        Self {
            config,
            pool,
            read_pool,
            redis,
            storage,
            cache: CacheService::new(),
            file_service,
            task_queue,
            search_index,
            embedding_service,
            zip_build_semaphore,
        }
    }
}
