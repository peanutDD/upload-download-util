//! # 文件数据访问层 - SQLx 实现
//!
//! 提供 `FilesRepository` trait 的 PostgreSQL/SQLx 实现，封装对 `files` 表的所有读写。
//!
//! ## 架构与约定
//!
//! - **表名**：`files`，主键 `id`（UUID），外键 `user_id` → `users(id)` ON DELETE CASCADE。
//! - **多租户**：所有查询均带 `user_id`，保证用户只能访问自己的记录。
//! - **错误**：SQLx 错误通过 `AppError::from` 转为 `AppError::Database`，由上层统一转 HTTP 状态码。
//! - **连接**：使用 `PgPool` 复用连接；`list` 内对单次查询使用 `SET LOCAL statement_timeout` 避免慢查询占用连接。
//!
//! ## 主要能力
//!
//! - 插入/按 ID 查/按内容哈希查（秒传）/按路径计数（去重删除）
//! - 分页列表（`list`）：支持搜索、MIME、分类、文件夹、日期、大小、排序，使用 `QueryBuilder` 动态拼接 WHERE
//! - 批量操作：删除、按 ID 取路径、按 ID 取实体、按 ID 汇总大小
//! - 存储用量、分类列表、批量更新分类

// =============================================================================
// 依赖与类型
// =============================================================================

use std::collections::HashMap;

use async_trait::async_trait; // 为 trait 方法提供 async fn 语法
use chrono::{DateTime, Utc}; // 日期解析与存库用 UTC
use sqlx::{postgres::PgRow, PgPool, Row}; // PgRow 用于 list 中取 total_count
use uuid::Uuid;

use crate::models::file::{File, FileListQuery, FileListResult}; // 实体与列表查询 DTO
use crate::repositories::traits::FilesRepository; // 本模块实现的 trait
use crate::utils::AppError; // 统一错误类型，SQLx 错误会转为 Database

// =============================================================================
// 结构体与构造
// =============================================================================

/// SQLx 实现的文件仓库
///
/// 持有 `PgPool` 句柄，通过 `Arc<dyn FilesRepository>` 注入到 Service 层。
/// 无内部可变状态，方法均为 `&self`，并发安全由连接池与数据库保证。
pub struct SqlxFilesRepo {
    write_pool: PgPool,
    read_pool: PgPool,
}

impl SqlxFilesRepo {
    pub fn new(pool: PgPool) -> Self {
        let read_pool = pool.clone();
        Self {
            write_pool: pool,
            read_pool,
        }
    }

    pub fn new_with_replica(write_pool: PgPool, read_pool: PgPool) -> Self {
        Self {
            write_pool,
            read_pool,
        }
    }

    fn r(&self) -> &PgPool {
        &self.read_pool
    }

    fn w(&self) -> &PgPool {
        &self.write_pool
    }
}

// =============================================================================
// 插入与秒传（上传落库、按内容哈希查、按路径计数）
// =============================================================================

#[async_trait]
impl FilesRepository for SqlxFilesRepo {
    /// 插入一条文件记录（上传落库）。
    ///
    /// **实现**：单条 `INSERT ... RETURNING *`，`file_size` 以 `i64` 存库。若违反唯一约束会返回
    /// `AppError::Database`。调用方一般在写入存储成功后调用，失败时需自行清理已写文件。
    async fn insert(
        &self,
        file_id: Uuid,
        user_id: Uuid,
        storage_filename: &str,
        original_filename: &str,
        file_path: &str,
        file_size: u64,
        mime_type: &str,
        storage_backend: &str,
        content_sha256: Option<&str>,
        folder_id: Option<Uuid>,
    ) -> Result<File, AppError> {
        sqlx::query_as::<_, File>(
            "INSERT INTO files (id, user_id, filename, original_filename, file_path, file_size, mime_type, storage_backend, content_sha256, folder_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *",
        )
        .bind(file_id) // 主键，由调用方生成 UUID
        .bind(user_id)
        .bind(storage_filename) // 存盘文件名（常带 UUID 前缀）
        .bind(original_filename) // 用户上传时的文件名
        .bind(file_path) // 存储层返回的路径，Local 或 S3 key
        .bind(file_size as i64) // 表字段为 BIGINT
        .bind(mime_type)
        .bind(storage_backend) // "local" | "s3"
        .bind(content_sha256) // 可选，秒传与去重用
        .bind(folder_id) // 可选，NULL 表示根目录
        .fetch_one(self.w()) // 取 RETURNING 的一行
        .await
        .map_err(AppError::from) // sqlx::Error -> AppError::Database
    }

    /// 按内容 SHA-256 与文件大小查找任意一条匹配记录，用于秒传：相同内容可复用已有存储并只插记录。
    ///
    /// **实现**：`WHERE content_sha256 = $1 AND file_size = $2 LIMIT 1`，返回 `Option<File>`。
    /// 未建 `(content_sha256, file_size)` 索引时可能全表扫描，数据量大时可考虑加索引。
    async fn find_by_content_hash_and_size(
        &self,
        content_sha256: &str,
        file_size: u64,
    ) -> Result<Option<File>, AppError> {
        sqlx::query_as::<_, File>(
            "SELECT * FROM files WHERE content_sha256 = $1 AND file_size = $2 AND deleted_at IS NULL LIMIT 1",
        )
        .bind(content_sha256) // 十六进制字符串
        .bind(file_size as i64)
        .fetch_optional(self.r()) // 0 或 1 行
        .await
        .map_err(AppError::from)
    }

    /// 统计引用同一 `file_path` 的记录数。
    ///
    /// **用途**：删除或秒传复用存储时，仅当计数为 0 才可安全删除物理文件，避免误删仍被引用的路径。
    /// **实现**：`COUNT(*)` + `WHERE file_path = $1`，返回 `u64`。
    async fn count_by_file_path(&self, file_path: &str) -> Result<u64, AppError> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*)::BIGINT FROM files WHERE file_path = $1")
                .bind(file_path)
                .fetch_one(self.r())
                .await?;
        Ok(count.0 as u64) // 单列元组取 .0，转为无符号
    }

    /// 批量统计多个 `file_path` 的引用数，一次查询减少 round-trip。
    ///
    /// **实现**：`SELECT file_path, COUNT(*) FROM files WHERE file_path = ANY($1) GROUP BY file_path`。
    /// 空 paths 直接返回空 HashMap；未出现在结果中的 path 表示 0 引用。
    async fn count_by_file_paths(
        &self,
        paths: &[String],
    ) -> Result<HashMap<String, u64>, AppError> {
        if paths.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT file_path, COUNT(*)::BIGINT FROM files WHERE file_path = ANY($1) GROUP BY file_path",
        )
        .bind(paths)
        .fetch_all(self.r())
        .await?;
        Ok(rows.into_iter().map(|(p, c)| (p, c as u64)).collect())
    }

    // =============================================================================
    // 单条查询与归属校验（下载/预览/删除前）
    // =============================================================================

    /// 按文件 ID 与用户 ID 查询单条记录，用于下载/预览/删除前校验归属。
    ///
    /// **实现**：`SELECT * FROM files WHERE id = $1 AND user_id = $2`，返回 `Option<File>`，
    /// 不存在或不属于该用户时为 `None`。
    async fn find_by_id(&self, file_id: Uuid, user_id: Uuid) -> Result<Option<File>, AppError> {
        sqlx::query_as::<_, File>(
            "SELECT * FROM files WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
        )
        .bind(file_id)
        .bind(user_id) // 双重条件防止越权
        .fetch_optional(self.r())
        .await
        .map_err(AppError::from)
    }

    async fn find_deleted_by_id(
        &self,
        file_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<File>, AppError> {
        sqlx::query_as::<_, File>(
            "SELECT * FROM files WHERE id = $1 AND user_id = $2 AND deleted_at IS NOT NULL",
        )
        .bind(file_id)
        .bind(user_id)
        .fetch_optional(self.r())
        .await
        .map_err(AppError::from)
    }

    /// 判断指定文件是否属于指定用户（仅做存在性检查，不返回实体）。
    ///
    /// **实现**：`SELECT id FROM files WHERE id = $1 AND user_id = $2`，有行则 `true`。
    /// 比 `find_by_id` 更轻量，适合仅做权限/归属判断的场景。
    async fn belongs_to_user(&self, file_id: Uuid, user_id: Uuid) -> Result<bool, AppError> {
        let result: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM files WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
        )
        .bind(file_id)
        .bind(user_id)
        .fetch_optional(self.r())
        .await?;
        Ok(result.is_some()) // 有行即属于该用户
    }

    async fn find_by_name_and_folder(
        &self,
        user_id: Uuid,
        original_filename: &str,
        folder_id: Option<Uuid>,
    ) -> Result<Option<File>, AppError> {
        let file = if let Some(fid) = folder_id {
            sqlx::query_as::<_, File>(
                "SELECT * FROM files WHERE user_id = $1 AND original_filename = $2 AND folder_id = $3 AND deleted_at IS NULL LIMIT 1",
            )
            .bind(user_id)
            .bind(original_filename)
            .bind(fid)
            .fetch_optional(self.r())
            .await?
        } else {
            sqlx::query_as::<_, File>(
                "SELECT * FROM files WHERE user_id = $1 AND original_filename = $2 AND folder_id IS NULL AND deleted_at IS NULL LIMIT 1",
            )
            .bind(user_id)
            .bind(original_filename)
            .fetch_optional(self.r())
            .await?
        };
        Ok(file)
    }

    async fn rename(
        &self,
        file_id: Uuid,
        user_id: Uuid,
        original_filename: &str,
    ) -> Result<File, AppError> {
        sqlx::query_as::<_, File>(
            r#"
            UPDATE files
            SET original_filename = $1, updated_at = CURRENT_TIMESTAMP
            WHERE id = $2 AND user_id = $3 AND deleted_at IS NULL
            RETURNING *
            "#,
        )
        .bind(original_filename)
        .bind(file_id)
        .bind(user_id)
        .fetch_one(self.w())
        .await
        .map_err(AppError::from)
    }

    // =============================================================================
    // 按文件夹列表（不分页，供文件夹树等场景）
    // =============================================================================

    /// 列出指定用户、指定文件夹下的文件（不分页，按创建时间倒序）。
    ///
    /// **实现**：单条 SQL，`folder_id = $2` 与 `$2 IS NULL AND folder_id IS NULL` 统一处理根目录与子文件夹。
    async fn list_by_folder(
        &self,
        user_id: Uuid,
        folder_id: Option<Uuid>,
    ) -> Result<Vec<File>, AppError> {
        sqlx::query_as::<_, File>(
            "SELECT * FROM files WHERE user_id = $1 AND deleted_at IS NULL AND original_filename NOT LIKE '._%' AND ( (folder_id IS NULL AND $2::uuid IS NULL) OR (folder_id = $2) ) ORDER BY created_at DESC",
        )
        .bind(user_id)
        .bind(folder_id)
        .fetch_all(self.r())
        .await
        .map_err(AppError::from)
    }

    // =============================================================================
    // 删除（单条 / 批量）
    // =============================================================================

    async fn soft_delete(&self, file_id: Uuid, user_id: Uuid) -> Result<u64, AppError> {
        let result = sqlx::query(
            "UPDATE files SET deleted_at = COALESCE(deleted_at, NOW()), updated_at = NOW() WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
        )
        .bind(file_id)
        .bind(user_id)
        .execute(self.w())
        .await?;
        Ok(result.rows_affected())
    }

    async fn soft_delete_batch(&self, ids: &[Uuid], user_id: Uuid) -> Result<u64, AppError> {
        if ids.is_empty() {
            return Ok(0);
        }
        let result = sqlx::query(
            "UPDATE files SET deleted_at = COALESCE(deleted_at, NOW()), updated_at = NOW() WHERE user_id = $1 AND id = ANY($2) AND deleted_at IS NULL",
        )
        .bind(user_id)
        .bind(ids)
        .execute(self.w())
        .await?;
        Ok(result.rows_affected())
    }

    async fn restore_deleted(&self, file_id: Uuid, user_id: Uuid) -> Result<File, AppError> {
        sqlx::query_as::<_, File>(
            r#"
            UPDATE files
            SET deleted_at = NULL, updated_at = NOW()
            WHERE id = $1 AND user_id = $2 AND deleted_at IS NOT NULL
            RETURNING *
            "#,
        )
        .bind(file_id)
        .bind(user_id)
        .fetch_one(self.w())
        .await
        .map_err(AppError::from)
    }

    async fn list_deleted(&self, user_id: Uuid) -> Result<Vec<File>, AppError> {
        sqlx::query_as::<_, File>(
            "SELECT * FROM files WHERE user_id = $1 AND deleted_at IS NOT NULL ORDER BY deleted_at DESC, id DESC",
        )
        .bind(user_id)
        .fetch_all(self.r())
        .await
        .map_err(AppError::from)
    }

    async fn hard_delete_deleted(&self, file_id: Uuid, user_id: Uuid) -> Result<u64, AppError> {
        let result = sqlx::query(
            "DELETE FROM files WHERE id = $1 AND user_id = $2 AND deleted_at IS NOT NULL",
        )
        .bind(file_id)
        .bind(user_id)
        .execute(self.w())
        .await?;
        Ok(result.rows_affected())
    }

    async fn hard_delete_deleted_batch(
        &self,
        ids: &[Uuid],
        user_id: Uuid,
    ) -> Result<Vec<File>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        sqlx::query_as::<_, File>(
            "DELETE FROM files WHERE user_id = $1 AND id = ANY($2) AND deleted_at IS NOT NULL RETURNING *",
        )
        .bind(user_id)
        .bind(ids)
        .fetch_all(self.w())
        .await
        .map_err(AppError::from)
    }

    async fn hard_delete_all_deleted(&self, user_id: Uuid) -> Result<Vec<File>, AppError> {
        sqlx::query_as::<_, File>(
            "DELETE FROM files WHERE user_id = $1 AND deleted_at IS NOT NULL RETURNING *",
        )
        .bind(user_id)
        .fetch_all(self.w())
        .await
        .map_err(AppError::from)
    }

    async fn purge_expired_deleted(
        &self,
        retention_days: i64,
        batch_limit: i64,
    ) -> Result<Vec<File>, AppError> {
        sqlx::query_as::<_, File>(
            r#"
            WITH picked AS (
                SELECT id
                FROM files
                WHERE deleted_at IS NOT NULL
                  AND deleted_at < NOW() - ($1::int * INTERVAL '1 day')
                ORDER BY deleted_at ASC
                LIMIT $2
            )
            DELETE FROM files f
            USING picked
            WHERE f.id = picked.id
            RETURNING f.*
            "#,
        )
        .bind(retention_days as i32)
        .bind(batch_limit)
        .fetch_all(self.w())
        .await
        .map_err(AppError::from)
    }

    async fn delete(&self, file_id: Uuid, user_id: Uuid) -> Result<u64, AppError> {
        let result = sqlx::query("DELETE FROM files WHERE id = $1 AND user_id = $2")
            .bind(file_id)
            .bind(user_id)
            .execute(self.w())
            .await?;
        Ok(result.rows_affected()) // 0 或 1
    }

    async fn delete_batch(&self, ids: &[Uuid], user_id: Uuid) -> Result<u64, AppError> {
        if ids.is_empty() {
            return Ok(0); // 避免 ANY('{}') 语义依赖
        }
        let result = sqlx::query("DELETE FROM files WHERE user_id = $1 AND id = ANY($2)")
            .bind(user_id)
            .bind(ids) // PostgreSQL 数组，只删本用户的 id
            .execute(self.w())
            .await?;
        Ok(result.rows_affected())
    }

    // =============================================================================
    // 存储用量与分类（配额、筛选器、批量更新分类）
    // =============================================================================

    /// 获取指定用户的存储使用量：总字节数、文件数量。
    ///
    /// **实现**：`SELECT COALESCE(SUM(file_size), 0), COUNT(*) FROM files WHERE user_id = $1`。
    /// 聚合无 GROUP BY 时始终返回一行，无文件时为 `(0, 0)`。用于配额展示与上传前校验。
    async fn get_storage_usage(&self, user_id: Uuid) -> Result<(i64, u64), AppError> {
        let (total_size, file_count): (i64, i64) = sqlx::query_as(
            "SELECT COALESCE(SUM(file_size)::BIGINT, 0), COUNT(*)::BIGINT FROM files WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(self.r()) // 聚合始终一行
        .await?;
        Ok((total_size, file_count as u64))
    }

    /// 列出该用户下所有出现过的非空分类名（去重、排序），用于筛选器下拉等。
    ///
    /// **实现**：`SELECT DISTINCT category ... AND category IS NOT NULL AND TRIM(category) != ''`。
    /// 分类字段已逐步被 `folder_id` 替代，接口保留兼容。
    async fn list_categories(&self, user_id: Uuid) -> Result<Vec<String>, AppError> {
        sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT category FROM files WHERE user_id = $1 AND deleted_at IS NULL AND category IS NOT NULL AND TRIM(category) != '' ORDER BY category",
        )
        .bind(user_id)
        .fetch_all(self.r()) // 返回单列多行，自动 Vec<String>
        .await
        .map_err(AppError::from)
    }

    /// 批量更新指定文件记录的 `category` 与 `updated_at`。
    ///
    /// **实现**：`UPDATE files SET category = $1, updated_at = $2 WHERE user_id = $3 AND id = ANY($4)`。
    /// 空 ids 直接返回 0。
    async fn update_category(
        &self,
        user_id: Uuid,
        ids: &[Uuid],
        category: Option<&str>,
        updated_at: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        if ids.is_empty() {
            return Ok(0);
        }
        let result = sqlx::query(
            "UPDATE files SET category = $1, updated_at = $2 WHERE user_id = $3 AND id = ANY($4) AND deleted_at IS NULL",
        )
        .bind(category) // Option<&str>，可置 NULL
        .bind(updated_at)
        .bind(user_id)
        .bind(ids)
        .execute(self.w())
        .await?;

        Ok(result.rows_affected())
    }

    // =============================================================================
    // 批量查询（按 ID 列表：汇总大小、取实体、取路径）
    // =============================================================================

    /// 统计指定 ID 集合中属于该用户的文件数量与总大小（用于批量下载前校验或展示）。
    ///
    /// **实现**：`SELECT COUNT(*), COALESCE(SUM(file_size), 0) FROM files WHERE user_id = $1 AND id = ANY($2)`，
    /// 返回 `(found_count, total_size)`。空 ids 直接返回 `(0, 0)`，避免 ANY('{}') 依赖。
    async fn sum_size_for_ids(&self, user_id: Uuid, ids: &[Uuid]) -> Result<(i64, i64), AppError> {
        if ids.is_empty() {
            return Ok((0, 0));
        }
        let (found_count, total_size): (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*)::BIGINT, COALESCE(SUM(file_size)::BIGINT, 0) \
             FROM files WHERE user_id = $1 AND id = ANY($2) AND deleted_at IS NULL",
        )
        .bind(user_id)
        .bind(ids)
        .fetch_one(self.r()) // 聚合结果始终一行
        .await?;
        Ok((found_count, total_size))
    }

    /// 按 ID 列表批量查询文件实体，仅返回属于该用户的记录，顺序不保证与 `ids` 一致。
    ///
    /// **实现**：`SELECT * FROM files WHERE user_id = $1 AND id = ANY($2)`。空 ids 直接返回空 Vec。
    async fn find_by_ids(&self, user_id: Uuid, ids: &[Uuid]) -> Result<Vec<File>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        sqlx::query_as::<_, File>(
            "SELECT * FROM files WHERE user_id = $1 AND id = ANY($2) AND deleted_at IS NULL",
        )
        .bind(user_id)
        .bind(ids)
        .fetch_all(self.r()) // 顺序与 ids 不一定一致
        .await
        .map_err(AppError::from)
    }

    /// 按 ID 列表批量查询 `(id, file_path)`，用于批量删除时先删存储再删 DB，避免 N+1。
    ///
    /// **实现**：`SELECT id, file_path FROM files WHERE user_id = $1 AND id = ANY($2)`。空 ids 直接返回空 Vec。
    async fn find_paths_by_ids(
        &self,
        user_id: Uuid,
        ids: &[Uuid],
    ) -> Result<Vec<(Uuid, String)>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, file_path FROM files WHERE user_id = $1 AND id = ANY($2) AND deleted_at IS NULL",
        )
        .bind(user_id)
        .bind(ids)
        .fetch_all(self.r()) // 仅两列，供批量删存储用
        .await
        .map_err(AppError::from)
    }

    // =============================================================================
    // 分页列表（文件列表页：搜索 / MIME / 分类 / 文件夹 / 日期 / 大小 / 排序）
    // =============================================================================

    /// 分页查询文件列表，支持多条件筛选与排序，返回当前页数据与总条数或游标。
    ///
    /// **返回值**：
    /// - 传统分页：`(Vec<File>, total_count)`，`total_count` 由 `COUNT(*) OVER()` 在单次查询中得出。
    /// - 游标分页：`(Vec<File>, next_cursor)`，`next_cursor` 为最后一条记录的排序字段值（字符串形式），如果已到末尾则为 `None`。
    ///
    /// **实现要点**：
    /// - 分页模式：
    ///   - 如果提供了 `cursor`，使用游标分页（keyset pagination）：`WHERE sort_column > cursor`（DESC）或 `WHERE sort_column < cursor`（ASC），不使用 `OFFSET`。
    ///   - 否则使用传统分页：`page` 默认 1，`limit` 默认 20、最大 100，`offset = (page - 1) * limit`。
    /// - 筛选：搜索（`original_filename`/`filename` ILIKE）、MIME（精确或前缀 `image/%`）、分类、文件夹、日期范围、大小范围；根目录（folder_id 为 null/root/空）不按 folder_id 过滤，返回该用户全部文件。
    /// - 排序：仅允许 `sort_by` ∈ { created_at, filename, file_size, type }，`sort_order` ∈ { asc, desc }，否则回退默认（created_at DESC）。
    ///   - `sort_by = type`：按预置类型顺序排序（image → gif → video → audio → pdf → text → zip → application → other），并在每个类型内按 created_at DESC、id DESC；该模式不支持游标分页。
    ///   - 其他字段：游标分页时，为确保唯一性，二级排序使用 `id`。
    /// - SQL 构建：使用 `QueryBuilder` 动态拼接 WHERE，避免手拼字符串与注入；传统分页的 `total_count` 用窗口函数一次查出。
    /// - 超时：本查询在独立事务中执行并 `SET LOCAL statement_timeout = '3s'`，避免慢查询长时间占用连接池。
    async fn list(&self, user_id: Uuid, query: FileListQuery) -> Result<FileListResult, AppError> {
        use chrono::{DateTime, NaiveDateTime, Utc};
        use sqlx::QueryBuilder;
        use uuid::Uuid;

        let limit = query.limit.unwrap_or(20).min(100); // 单页最多 100 条

        // ---- 筛选条件预解析（空字符串视为未传，不参与 WHERE） ----
        let search_pattern: Option<String> = query
            .search
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{}%", s)); // ILIKE 用前后 % 包裹

        // MIME：以 / 结尾（如 image/）则按前缀 LIKE，否则精确匹配
        let mime_type_filter: Option<String> = query
            .mime_type
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|s| {
                if s.ends_with('/') {
                    format!("{}%", s) // 如 image/% 匹配 image/png 等
                } else {
                    s.to_string()
                }
            });
        let mime_type_is_prefix = query
            .mime_type
            .as_deref()
            .map(|s| s.ends_with('/'))
            .unwrap_or(false); // 决定下面用 LIKE 还是 =

        let category_filter_uncategorized = query
            .category
            .as_deref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(false); // 传空字符串表示筛「未分类」
        let category_filter_exact: Option<String> = query.category.as_ref().and_then(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string()) // 精确分类名
            }
        });

        let folder_id_filter: Option<Option<Uuid>> = match query.folder_id.as_deref() {
            None => None,
            Some(s) => {
                let t = s.trim();
                if t.is_empty() || t.eq_ignore_ascii_case("null") || t == "root" {
                    Some(None)
                } else {
                    Uuid::parse_str(t).ok().map(Some)
                }
            }
        };

        // 日期范围：支持 YYYY-MM-DD 或 RFC3339；date_to 解析为当日 23:59:59 以包含整天
        let date_from: Option<DateTime<Utc>> = query
            .date_from
            .as_deref()
            .and_then(|s| {
                NaiveDateTime::parse_from_str(s, "%Y-%m-%d")
                    .ok()
                    .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
            })
            .or_else(|| {
                query.date_from.as_deref().and_then(|s| {
                    DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                })
            });

        let date_to: Option<DateTime<Utc>> = query
            .date_to
            .as_deref()
            .and_then(|s| {
                NaiveDateTime::parse_from_str(s, "%Y-%m-%d")
                    .ok()
                    .and_then(|dt| {
                        dt.date()
                            .and_hms_opt(23, 59, 59) // 当天结束，包含整天
                            .map(|end_of_day| {
                                DateTime::<Utc>::from_naive_utc_and_offset(end_of_day, Utc)
                            })
                    })
            })
            .or_else(|| {
                query.date_to.as_deref().and_then(|s| {
                    DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                })
            });

        // ---- 排序字段与方向（白名单，非法值回退默认） ----
        let sort_by = query.sort_by.as_deref().unwrap_or("created_at");
        let sort_direction = match query.sort_order.as_deref() {
            Some("asc") => "ASC",
            Some("desc") | None => "DESC",
            _ => "DESC",
        };
        let sort_column = match sort_by {
            "filename" => "original_filename", // API 用 filename，库列名为 original_filename
            "file_size" => "file_size",
            "created_at" => "created_at",
            "type" => "CASE \
                WHEN lower(mime_type) LIKE 'image/gif%' THEN 2 \
                WHEN lower(mime_type) LIKE 'image/%' THEN 1 \
                WHEN lower(mime_type) LIKE 'video/%' THEN 3 \
                WHEN lower(mime_type) LIKE 'audio/%' THEN 4 \
                WHEN lower(mime_type) = 'application/pdf' THEN 5 \
                WHEN lower(mime_type) LIKE 'text/%' THEN 6 \
                WHEN lower(mime_type) = 'application/zip' OR lower(mime_type) = 'application/x-zip-compressed' THEN 7 \
                WHEN lower(mime_type) LIKE 'application/%' THEN 8 \
                ELSE 99 \
            END",
            _ => "created_at", // 非法值回退默认，仅白名单不引入注入
        };

        let use_cursor_pagination = (query.cursor.is_some()
            || matches!(query.pagination.as_deref(), Some("cursor")))
            && sort_by != "type";

        // ---- 游标解析（如果使用游标分页） ----
        enum CursorValue {
            DateTime(DateTime<Utc>),
            String(String),
            Int64(i64),
        }

        #[derive(serde::Deserialize, serde::Serialize)]
        struct CursorTokenV1 {
            v: u8,
            sort_by: String,
            sort_order: String,
            value: String,
            id: Uuid,
        }

        fn parse_cursor_value(sort_column: &str, cursor_str: &str) -> Option<CursorValue> {
            match sort_column {
                "created_at" => DateTime::parse_from_rfc3339(cursor_str)
                    .ok()
                    .map(|dt| CursorValue::DateTime(dt.with_timezone(&Utc)))
                    .or_else(|| {
                        NaiveDateTime::parse_from_str(cursor_str, "%Y-%m-%d %H:%M:%S%.f")
                            .ok()
                            .map(|dt| {
                                CursorValue::DateTime(DateTime::<Utc>::from_naive_utc_and_offset(
                                    dt, Utc,
                                ))
                            })
                    }),
                "original_filename" => Some(CursorValue::String(cursor_str.to_string())),
                "file_size" => cursor_str.parse::<i64>().ok().map(CursorValue::Int64),
                _ => None,
            }
        }

        let (cursor_value, cursor_id, cursor_has_id): (Option<CursorValue>, Option<Uuid>, bool) =
            if use_cursor_pagination {
                let cursor_str = query.cursor.as_deref().unwrap_or("");
                if let Ok(token) = serde_json::from_str::<CursorTokenV1>(cursor_str) {
                    if token.v != 1 {
                        return Err(AppError::Validation("无效的 cursor 版本".to_string()));
                    }

                    let expected_sort_by = match sort_column {
                        "created_at" => "created_at",
                        "original_filename" => "filename",
                        "file_size" => "file_size",
                        _ => "created_at",
                    };
                    let expected_sort_order = if sort_direction == "ASC" {
                        "asc"
                    } else {
                        "desc"
                    };
                    if token.sort_by != expected_sort_by || token.sort_order != expected_sort_order
                    {
                        return Err(AppError::Validation(
                            "cursor 与 sort_by/sort_order 不匹配".to_string(),
                        ));
                    }
                    (
                        parse_cursor_value(sort_column, &token.value),
                        Some(token.id),
                        true,
                    )
                } else {
                    (parse_cursor_value(sort_column, cursor_str), None, false)
                }
            } else {
                (None, None, false)
            };

        // ---- 动态拼接 SELECT + WHERE + ORDER BY + LIMIT/OFFSET 或 LIMIT（游标分页） ----
        // 传统分页使用 COUNT(*) OVER() 获取总条数（除非 include_total=false），游标分页不需要
        let should_count_total = !use_cursor_pagination && query.include_total.unwrap_or(true);

        let mut qb: QueryBuilder<sqlx::Postgres> = if should_count_total {
            QueryBuilder::new(
                "SELECT id, user_id, filename, original_filename, file_path, file_size, mime_type, storage_backend, category, folder_id, content_sha256, created_at, updated_at, deleted_at, COUNT(*) OVER() AS total_count FROM files WHERE user_id = "
            )
        } else {
            QueryBuilder::new(
                "SELECT id, user_id, filename, original_filename, file_path, file_size, mime_type, storage_backend, category, folder_id, content_sha256, created_at, updated_at, deleted_at FROM files WHERE user_id = "
            )
        };
        qb.push_bind(user_id); // 首绑：user_id，后续条件用 push + push_bind 避免 SQL 注入
        qb.push(" AND deleted_at IS NULL");
        qb.push(" AND original_filename NOT LIKE '._%'");

        if category_filter_uncategorized {
            qb.push(" AND (category IS NULL OR category = '' OR TRIM(category) = '')");
        } else if let Some(cat) = &category_filter_exact {
            qb.push(" AND category = ");
            qb.push_bind(cat);
        }

        if let Some(folder_opt) = &folder_id_filter {
            if let Some(fid) = folder_opt {
                qb.push(" AND folder_id = ");
                qb.push_bind(*fid);
            } else {
                qb.push(" AND folder_id IS NULL"); // 显式请求根目录时
            }
        }

        if let Some(sp) = &search_pattern {
            qb.push(" AND (original_filename ILIKE ");
            qb.push_bind(sp);
            qb.push(" OR filename ILIKE ");
            qb.push_bind(sp);
            qb.push(")");
        }

        if let Some(mt) = &mime_type_filter {
            if mime_type_is_prefix {
                qb.push(" AND mime_type LIKE ");
            } else {
                qb.push(" AND mime_type = ");
            }
            qb.push_bind(mt);
        }

        if let Some(df) = date_from {
            qb.push(" AND created_at >= ");
            qb.push_bind(df);
        }

        if let Some(dt) = date_to {
            qb.push(" AND created_at <= ");
            qb.push_bind(dt);
        }

        if let Some(size_min) = query.size_min {
            qb.push(" AND file_size >= ");
            qb.push_bind(size_min);
        }

        if let Some(size_max) = query.size_max {
            qb.push(" AND file_size <= ");
            qb.push_bind(size_max);
        }

        // ---- 游标分页：添加 WHERE keyset 条件 ----
        if use_cursor_pagination {
            if let Some(cursor) = &cursor_value {
                if cursor_has_id {
                    let Some(cid) = cursor_id else {
                        return Err(AppError::Validation("无效的 cursor".to_string()));
                    };
                    qb.push(" AND (");
                    qb.push(sort_column);
                    qb.push(", id) ");
                    if sort_direction == "DESC" {
                        qb.push(" < (");
                    } else {
                        qb.push(" > (");
                    }
                    match cursor {
                        CursorValue::DateTime(dt) => qb.push_bind(*dt),
                        CursorValue::String(s) => qb.push_bind(s),
                        CursorValue::Int64(n) => qb.push_bind(*n),
                    };
                    qb.push(", ");
                    qb.push_bind(cid);
                    qb.push(")");
                } else {
                    qb.push(" AND ");
                    qb.push(sort_column);
                    if sort_direction == "DESC" {
                        qb.push(" < ");
                    } else {
                        qb.push(" > ");
                    }
                    match cursor {
                        CursorValue::DateTime(dt) => qb.push_bind(*dt),
                        CursorValue::String(s) => qb.push_bind(s),
                        CursorValue::Int64(n) => qb.push_bind(*n),
                    };
                }
            }
        }

        qb.push(" ORDER BY ");
        qb.push(sort_column); // 已白名单，非用户输入
        qb.push(" ");
        qb.push(sort_direction);
        if sort_by == "type" {
            qb.push(", created_at DESC, id DESC");
        } else {
            qb.push(", id ");
            qb.push(sort_direction);
        }
        qb.push(" LIMIT ");
        let limit_plus_one: i64 = if use_cursor_pagination {
            (limit as i64) + 1
        } else {
            limit as i64
        };
        qb.push_bind(limit_plus_one);

        // 传统分页才使用 OFFSET
        if !use_cursor_pagination {
            let page = query.page.unwrap_or(1);
            let offset = (page - 1) * limit;
            qb.push(" OFFSET ");
            qb.push_bind(offset as i64);
        }

        // 在独立事务中执行列表查询，并设置 3s 超时，避免慢查询占用连接池影响其他请求
        let mut tx = self.r().begin().await?;
        sqlx::query("SET LOCAL statement_timeout = '3s'")
            .execute(&mut *tx)
            .await?;

        let rows: Vec<PgRow> = qb.build().fetch_all(&mut *tx).await?; // 执行拼接后的 SQL
        tx.commit().await?; // 提交事务（SET LOCAL 仅本事务有效，提交后连接归还池）

        // 传统分页：先从首行取窗口函数结果 total_count（每行相同），然后再转换为 files
        let total: Option<i64> = if should_count_total {
            rows.first()
                .and_then(|row| row.try_get::<i64, _>("total_count").ok())
                .or(Some(0)) // 无行时 0
        } else {
            None
        };

        // 将 PgRow 映射为 File 实体（SELECT 列与 File 字段一致，FromRow 可反序列化）
        let files: Vec<File> = rows
            .into_iter()
            .map(|row| sqlx::FromRow::from_row(&row).map_err(AppError::Database)) // 列名与 File 字段对应
            .collect::<Result<Vec<_>, _>>()?;

        // 计算返回值：传统分页返回 total_count，游标分页返回 next_cursor
        if use_cursor_pagination {
            let has_more = files.len() > limit as usize;
            let mut files = files;
            if has_more {
                files.truncate(limit as usize);
            }

            let next_cursor = if has_more {
                files.last().map(|file| {
                    let (sort_by, sort_order) = (
                        query
                            .sort_by
                            .clone()
                            .unwrap_or_else(|| "created_at".to_string()),
                        query
                            .sort_order
                            .clone()
                            .unwrap_or_else(|| "desc".to_string()),
                    );
                    let value = match sort_column {
                        "created_at" => file.created_at.to_rfc3339(),
                        "original_filename" => file.original_filename.clone(),
                        "file_size" => file.file_size.to_string(),
                        _ => String::new(),
                    };
                    serde_json::to_string(&CursorTokenV1 {
                        v: 1,
                        sort_by,
                        sort_order,
                        value,
                        id: file.id,
                    })
                    .unwrap_or_default()
                })
            } else {
                None
            };
            Ok(FileListResult {
                files,
                total: None,
                next_cursor,
            })
        } else {
            Ok(FileListResult {
                files,
                total,
                next_cursor: None,
            })
        }
    }
}
