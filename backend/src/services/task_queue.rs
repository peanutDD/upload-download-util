// =============================================================================
// 依赖：标准库
// =============================================================================
use std::sync::Arc;
use std::time::Instant;

// =============================================================================
// 依赖：三方库
// =============================================================================
use async_trait::async_trait;
use metrics::{counter, histogram};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, PgPool};
use uuid::Uuid;

// =============================================================================
// 依赖：内部模块
// =============================================================================
use crate::models::file::File;
use crate::services::fulltext_indexer::FulltextIndexer;
use crate::utils::AppError;

// =============================================================================
// 常量与类型
// =============================================================================

/// 单个后台任务的最大自动重试次数。
///
/// - attempts 在 dequeue 时自增一次，因此 attempts = 1 表示第一次真实执行。
/// - 当 attempts >= MAX_ATTEMPTS 时，本次失败会被标记为最终失败（死信），不再自动重试。
const MAX_ATTEMPTS: i32 = 3;

// 任务状态枚举（存储层使用字符串落库）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

impl TaskStatus {
    fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Succeeded => "succeeded",
            TaskStatus::Failed => "failed",
        }
    }
}

// 后台任务记录（队列核心结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundTask {
    pub id: Uuid,
    pub task_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub attempts: i32,
}

// 管理后台查看的任务 DTO
#[derive(Debug, Clone, Serialize)]
pub struct AdminTask {
    pub id: Uuid,
    pub task_type: String,
    pub status: String,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub dedupe_key: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub next_run_at: chrono::DateTime<chrono::Utc>,
    pub locked_until: Option<chrono::DateTime<chrono::Utc>>,
}

// 队列深度统计
#[derive(Debug, Clone, Copy)]
pub struct TaskQueueDepth {
    pub pending_total: i64,
    pub pending_ready: i64,
    pub running: i64,
    pub failed: i64,
}

// 任务队列实现（基于 Postgres）
#[derive(Debug, Clone)]
pub struct TaskQueue {
    pool: Arc<PgPool>,
}

// =============================================================================
// Trait 定义
// =============================================================================

#[async_trait]
pub trait TaskQueueProvider: Send + Sync {
    async fn enqueue_task(
        &self,
        task_type: &str,
        payload: serde_json::Value,
        dedupe_key: Option<&str>,
    ) -> Result<BackgroundTask, AppError>;

    async fn dequeue_pending_task(
        &self,
        task_type: &str,
    ) -> Result<Option<BackgroundTask>, AppError>;

    async fn mark_succeeded(&self, task_id: Uuid) -> Result<(), AppError>;

    async fn mark_failed(&self, task_id: Uuid, msg: &str) -> Result<(), AppError>;

    async fn mark_pending_with_error(
        &self,
        task_id: Uuid,
        attempts: i32,
        msg: &str,
    ) -> Result<(), AppError>;

    async fn requeue_stuck_tasks(&self, task_type: &str, limit: i64) -> Result<i64, AppError>;

    async fn get_queue_depth(&self, task_type: &str) -> Result<TaskQueueDepth, AppError>;

    async fn list_tasks(
        &self,
        task_type: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AdminTask>, AppError>;

    async fn retry_task(&self, task_id: Uuid) -> Result<bool, AppError>;
}

pub type DynTaskQueue = Arc<dyn TaskQueueProvider>;

// =============================================================================
// 内部工具
// =============================================================================

fn retry_delay_secs(attempts: i32, task_id: Uuid) -> i64 {
    let exp = (attempts - 1).clamp(0, 10) as u32;
    let base = 5_i64.saturating_mul(2_i64.saturating_pow(exp));
    let capped = base.min(300);

    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    task_id.hash(&mut h);
    attempts.hash(&mut h);
    let jitter = (h.finish() % 4) as i64;
    capped + jitter
}

const TASK_LEASE_SECS: i64 = 600;
type AdminTaskRow = (
    Uuid,
    String,
    String,
    i32,
    Option<String>,
    Option<String>,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
    chrono::DateTime<chrono::Utc>,
    Option<chrono::DateTime<chrono::Utc>>,
);

// =============================================================================
// TaskQueue 主实现
// =============================================================================

impl TaskQueue {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// 创建或复用一个后台任务。
    ///
    /// 若提供 `dedupe_key`，则在存在相同 dedupe_key 且仍为 pending/running 的任务时直接返回该任务，
    /// 避免对同一业务实体重复排队。
    pub async fn enqueue_task(
        &self,
        task_type: &str,
        payload: serde_json::Value,
        dedupe_key: Option<&str>,
    ) -> Result<BackgroundTask, AppError> {
        let id = Uuid::new_v4();
        let status = TaskStatus::Pending.as_str().to_string();
        if let Some(key) = dedupe_key {
            let dedupe = key.to_string();
            let inserted: Option<(Uuid, String, serde_json::Value, String, i32)> = query_as(
                "INSERT INTO background_tasks (id, task_type, payload, status, attempts, dedupe_key)
                 VALUES ($1, $2, $3, $4, 0, $5)
                 ON CONFLICT (task_type, dedupe_key)
                   WHERE dedupe_key IS NOT NULL AND status IN ('pending', 'running')
                 DO NOTHING
                 RETURNING id, task_type, payload, status, attempts",
            )
            .bind(id)
            .bind(task_type)
            .bind(&payload)
            .bind(&status)
            .bind(&dedupe)
            .fetch_optional(&*self.pool)
            .await
            .map_err(AppError::from)?;

            if let Some(rec) = inserted {
                return Ok(BackgroundTask {
                    id: rec.0,
                    task_type: rec.1,
                    payload: rec.2,
                    status: rec.3,
                    attempts: rec.4,
                });
            }

            if let Some(existing) = self
                .find_active_by_dedupe_key(task_type, &dedupe)
                .await
                .map_err(AppError::from)?
            {
                return Ok(existing);
            }
        }

        let dedupe = dedupe_key.map(|s| s.to_string());
        let rec: (Uuid, String, serde_json::Value, String, i32) = query_as(
            "INSERT INTO background_tasks (id, task_type, payload, status, attempts, dedupe_key)
             VALUES ($1, $2, $3, $4, 0, $5)
             RETURNING id, task_type, payload, status, attempts",
        )
        .bind(id)
        .bind(task_type)
        .bind(&payload)
        .bind(&status)
        .bind(dedupe)
        .fetch_one(&*self.pool)
        .await
        .map_err(AppError::from)?;

        Ok(BackgroundTask {
            id: rec.0,
            task_type: rec.1,
            payload: rec.2,
            status: rec.3,
            attempts: rec.4,
        })
    }

    /// 从队列中取出一个待处理任务（使用 FOR UPDATE SKIP LOCKED）。
    pub async fn dequeue_pending_task(
        &self,
        task_type: &str,
    ) -> Result<Option<BackgroundTask>, AppError> {
        // 事务内抢占任务，避免并发重复消费
        let mut tx = self.pool.begin().await.map_err(AppError::from)?;

        let row: Option<(Uuid, serde_json::Value, i32)> = query_as(
            "SELECT id, payload, attempts
             FROM background_tasks
             WHERE task_type = $1
               AND status = 'pending'
               AND next_run_at <= NOW()
             ORDER BY created_at
             FOR UPDATE SKIP LOCKED
             LIMIT 1",
        )
        .bind(task_type)
        .fetch_optional(&mut *tx)
        .await
        .map_err(AppError::from)?;

        // 无任务时直接提交事务并返回
        let Some((id, payload, attempts)) = row else {
            tx.commit().await.map_err(AppError::from)?;
            return Ok(None);
        };

        // 标记 running + 加 lease，避免长任务被重复领取
        query(
            "UPDATE background_tasks
             SET status = 'running',
                 attempts = attempts + 1,
                 started_at = NOW(),
                 updated_at = NOW(),
                 locked_until = NOW() + make_interval(secs => $2)
             WHERE id = $1",
        )
        .bind(id)
        .bind(TASK_LEASE_SECS)
        .execute(&mut *tx)
        .await
        .map_err(AppError::from)?;

        tx.commit().await.map_err(AppError::from)?;

        Ok(Some(BackgroundTask {
            id,
            task_type: task_type.to_string(),
            payload,
            status: TaskStatus::Running.as_str().to_string(),
            attempts: attempts + 1,
        }))
    }

    pub async fn mark_succeeded(&self, id: Uuid) -> Result<(), AppError> {
        let status = TaskStatus::Succeeded.as_str();

        // 成功完成：写入完成时间
        query(
            "UPDATE background_tasks
             SET status = $2, completed_at = NOW(), updated_at = NOW(), locked_until = NULL
             WHERE id = $1",
        )
        .bind(id)
        .bind(status)
        .execute(&*self.pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn mark_failed(&self, id: Uuid, error: &str) -> Result<(), AppError> {
        let status = TaskStatus::Failed.as_str();

        // 最终失败：记录错误信息与完成时间
        query(
            "UPDATE background_tasks
             SET status = $2, last_error = $3, completed_at = NOW(), updated_at = NOW(), locked_until = NULL
             WHERE id = $1",
        )
        .bind(id)
        .bind(status)
        .bind(error)
        .execute(&*self.pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    /// 将任务重新标记为 pending 并记录错误信息，供后续自动重试。
    pub async fn mark_pending_with_error(
        &self,
        id: Uuid,
        attempts: i32,
        error: &str,
    ) -> Result<(), AppError> {
        // 可重试失败：回退为 pending 并设置下次执行时间
        let delay_secs = retry_delay_secs(attempts, id);
        query(
            "UPDATE background_tasks
             SET status = 'pending',
                 last_error = $2,
                 updated_at = NOW(),
                 next_run_at = NOW() + make_interval(secs => $3),
                 locked_until = NULL
             WHERE id = $1",
        )
        .bind(id)
        .bind(error)
        .bind(delay_secs)
        .execute(&*self.pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn get_queue_depth(&self, task_type: &str) -> Result<TaskQueueDepth, AppError> {
        // 汇总所有状态数量
        let rows: Vec<(String, i64)> = query_as(
            "SELECT status, COUNT(*)::bigint
             FROM background_tasks
             WHERE task_type = $1
             GROUP BY status",
        )
        .bind(task_type)
        .fetch_all(&*self.pool)
        .await
        .map_err(AppError::from)?;

        let mut pending_total = 0_i64;
        let mut running = 0_i64;
        let mut failed = 0_i64;
        for (status, count) in rows {
            match status.as_str() {
                "pending" => pending_total = count,
                "running" => running = count,
                "failed" => failed = count,
                _ => {}
            }
        }

        // 仅统计可立即执行的 pending
        let pending_ready: (i64,) = query_as(
            "SELECT COUNT(*)::bigint
             FROM background_tasks
             WHERE task_type = $1
               AND status = 'pending'
               AND next_run_at <= NOW()",
        )
        .bind(task_type)
        .fetch_one(&*self.pool)
        .await
        .map_err(AppError::from)?;

        Ok(TaskQueueDepth {
            pending_total,
            pending_ready: pending_ready.0,
            running,
            failed,
        })
    }

    pub async fn requeue_stuck_tasks(&self, task_type: &str, limit: i64) -> Result<i64, AppError> {
        // 回收超时的 running 任务，避免永久卡死
        let res = query(
            "WITH picked AS (
               SELECT id
               FROM background_tasks
               WHERE task_type = $1
                 AND status = 'running'
                 AND locked_until IS NOT NULL
                 AND locked_until < NOW()
               ORDER BY locked_until
               LIMIT $2
             )
             UPDATE background_tasks t
             SET status = 'pending',
                 last_error = 'task lease expired',
                 updated_at = NOW(),
                 next_run_at = NOW(),
                 locked_until = NULL
             FROM picked
             WHERE t.id = picked.id",
        )
        .bind(task_type)
        .bind(limit)
        .execute(&*self.pool)
        .await
        .map_err(AppError::from)?;

        Ok(res.rows_affected() as i64)
    }

    pub async fn list_tasks(
        &self,
        task_type: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AdminTask>, AppError> {
        // 管理后台列表：支持 type/status 过滤
        let rows: Vec<AdminTaskRow> = query_as(
            "SELECT id,
                    task_type,
                    status,
                    attempts,
                    last_error,
                    dedupe_key,
                    created_at,
                    updated_at,
                    started_at,
                    completed_at,
                    next_run_at,
                    locked_until
             FROM background_tasks
             WHERE ($1::text IS NULL OR task_type = $1)
               AND ($2::text IS NULL OR status = $2)
             ORDER BY created_at DESC
             LIMIT $3 OFFSET $4",
        )
        .bind(task_type)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(AppError::from)?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    task_type,
                    status,
                    attempts,
                    last_error,
                    dedupe_key,
                    created_at,
                    updated_at,
                    started_at,
                    completed_at,
                    next_run_at,
                    locked_until,
                )| AdminTask {
                    id,
                    task_type,
                    status,
                    attempts,
                    last_error,
                    dedupe_key,
                    created_at,
                    updated_at,
                    started_at,
                    completed_at,
                    next_run_at,
                    locked_until,
                },
            )
            .collect())
    }

    pub async fn retry_task(&self, id: Uuid) -> Result<bool, AppError> {
        // 重置失败任务到初始状态
        let res = query(
            "UPDATE background_tasks
             SET status = 'pending',
                 attempts = 0,
                 last_error = NULL,
                 started_at = NULL,
                 completed_at = NULL,
                 updated_at = NOW(),
                 next_run_at = NOW(),
                 locked_until = NULL
             WHERE id = $1",
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(AppError::from)?;
        Ok(res.rows_affected() > 0)
    }

    async fn find_active_by_dedupe_key(
        &self,
        task_type: &str,
        dedupe_key: &str,
    ) -> Result<Option<BackgroundTask>, sqlx::Error> {
        // 查询可复用的 pending/running 任务
        let rec: Option<(Uuid, String, serde_json::Value, String, i32)> = query_as(
            "SELECT id, task_type, payload, status, attempts
             FROM background_tasks
             WHERE task_type = $1
               AND dedupe_key = $2
               AND status IN ('pending', 'running')
             ORDER BY created_at
             LIMIT 1",
        )
        .bind(task_type)
        .bind(dedupe_key)
        .fetch_optional(&*self.pool)
        .await?;

        Ok(rec.map(|r| BackgroundTask {
            id: r.0,
            task_type: r.1,
            payload: r.2,
            status: r.3,
            attempts: r.4,
        }))
    }
}

// =============================================================================
// Trait 适配实现
// =============================================================================

#[async_trait]
impl TaskQueueProvider for TaskQueue {
    async fn enqueue_task(
        &self,
        task_type: &str,
        payload: serde_json::Value,
        dedupe_key: Option<&str>,
    ) -> Result<BackgroundTask, AppError> {
        TaskQueue::enqueue_task(self, task_type, payload, dedupe_key).await
    }

    async fn dequeue_pending_task(
        &self,
        task_type: &str,
    ) -> Result<Option<BackgroundTask>, AppError> {
        TaskQueue::dequeue_pending_task(self, task_type).await
    }

    async fn mark_succeeded(&self, task_id: Uuid) -> Result<(), AppError> {
        TaskQueue::mark_succeeded(self, task_id).await
    }

    async fn mark_failed(&self, task_id: Uuid, msg: &str) -> Result<(), AppError> {
        TaskQueue::mark_failed(self, task_id, msg).await
    }

    async fn mark_pending_with_error(
        &self,
        task_id: Uuid,
        attempts: i32,
        msg: &str,
    ) -> Result<(), AppError> {
        TaskQueue::mark_pending_with_error(self, task_id, attempts, msg).await
    }

    async fn requeue_stuck_tasks(&self, task_type: &str, limit: i64) -> Result<i64, AppError> {
        TaskQueue::requeue_stuck_tasks(self, task_type, limit).await
    }

    async fn get_queue_depth(&self, task_type: &str) -> Result<TaskQueueDepth, AppError> {
        TaskQueue::get_queue_depth(self, task_type).await
    }

    async fn list_tasks(
        &self,
        task_type: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AdminTask>, AppError> {
        TaskQueue::list_tasks(self, task_type, status, limit, offset).await
    }

    async fn retry_task(&self, task_id: Uuid) -> Result<bool, AppError> {
        TaskQueue::retry_task(self, task_id).await
    }
}

// =============================================================================
// Worker：GIF 预览转码
// =============================================================================

/// GIF 预览转码任务的 payload 模式。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GifPreviewPayload {
    pub file_id: Uuid,
    pub user_id: Uuid,
    pub storage_backend: String,
    pub source_path: String,
}

/// 简单的 GIF 预览 Worker：从队列中取出 `gif_preview` 任务并执行转码。
#[tracing::instrument(skip(state, transcode_semaphore, task_type_semaphore), fields(task_id))]
pub async fn run_gif_preview_worker(
    state: &crate::AppState,
    transcode_semaphore: std::sync::Arc<tokio::sync::Semaphore>,
    task_type_semaphore: Option<std::sync::Arc<tokio::sync::Semaphore>>,
) -> Result<(), AppError> {
    // 从队列抢占一条待处理任务
    let task = match state.task_queue.dequeue_pending_task("gif_preview").await? {
        Some(t) => t,
        None => return Ok(()),
    };

    tracing::Span::current().record("task_id", tracing::field::display(task.id));

    // 解析 payload
    let payload: GifPreviewPayload = serde_json::from_value(task.payload.clone())
        .map_err(|e| AppError::File(format!("解析 gif_preview payload 失败: {}", e)))?;

    let started_at = Instant::now();

    // 仅支持本地存储；S3 等后端后续扩展
    if payload.storage_backend != "local" {
        state
            .task_queue
            .mark_failed(
                task.id,
                "gif_preview only supports local storage backend for now",
            )
            .await?;
        return Ok(());
    }

    // 查询最新文件记录，避免任务创建后文件被删除或移动
    let file: File = match state
        .file_service
        .get_file(payload.file_id, payload.user_id)
        .await
    {
        Ok(f) => f,
        Err(AppError::NotFound) => {
            state
                .task_queue
                .mark_failed(
                    task.id,
                    "gif_preview source file not found (deleted or moved)",
                )
                .await?;
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    // 获取全局并发配额与任务类型配额
    let _global_permit = transcode_semaphore
        .acquire_owned()
        .await
        .map_err(|_| AppError::Internal)?;
    let _type_permit = if let Some(sema) = task_type_semaphore {
        Some(sema.acquire_owned().await.map_err(|_| AppError::Internal)?)
    } else {
        None
    };

    // 执行转码
    match state.file_service.transcode_gif_to_mp4(&file).await {
        Ok(_path) => {
            let elapsed = started_at.elapsed();
            // 成功指标
            counter!(
                "transcode_jobs_total",
                "task_type" => "gif_preview",
                "status" => "succeeded"
            )
            .increment(1);
            histogram!(
                "transcode_duration_seconds",
                "task_type" => "gif_preview"
            )
            .record(elapsed.as_secs_f64());

            // 成功日志
            tracing::info!(
                task_id = %task.id,
                user_id = %payload.user_id,
                file_id = %payload.file_id,
                duration_ms = elapsed.as_millis() as u64,
                "gif_preview transcode succeeded"
            );

            state.task_queue.mark_succeeded(task.id).await?;
        }
        Err(e) => {
            let elapsed = started_at.elapsed();
            let msg = format!("gif_preview transcode failed: {}", e);

            // 失败指标
            counter!(
                "transcode_jobs_total",
                "task_type" => "gif_preview",
                "status" => "failed"
            )
            .increment(1);
            histogram!(
                "transcode_duration_seconds",
                "task_type" => "gif_preview"
            )
            .record(elapsed.as_secs_f64());

            // 失败日志
            tracing::error!(
                task_id = %task.id,
                user_id = %payload.user_id,
                file_id = %payload.file_id,
                duration_ms = elapsed.as_millis() as u64,
                error = %e,
                "gif_preview transcode failed"
            );

            // 若尚未达到最大重试次数，则重新排回 pending 队列；否则标记为最终失败（死信）。
            if task.attempts < MAX_ATTEMPTS {
                state
                    .task_queue
                    .mark_pending_with_error(task.id, task.attempts, &msg)
                    .await?;
            } else {
                state.task_queue.mark_failed(task.id, &msg).await?;
            }
        }
    }

    Ok(())
}

// =============================================================================
// Worker：HLS 视频预览转码
// =============================================================================

/// HLS 预览转码任务的 payload 模式。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlsPreviewPayload {
    pub file_id: Uuid,
    pub user_id: Uuid,
    pub storage_backend: String,
}

/// HLS 预览 Worker：从队列取出 `hls_preview` 任务并调用 `ensure_hls_ready`。
///
/// 设计与 `run_gif_preview_worker` 完全对齐：
/// - 全局转码并发配额（`transcode_semaphore`）
/// - 任务类型级并发配额（`task_type_semaphore`，对应 `TASK_TYPE_CONCURRENCY_hls_preview`）
/// - 指数退避重试（`mark_pending_with_error`）直至 `MAX_ATTEMPTS` 后死信
/// - Prometheus 指标（`transcode_jobs_total` / `transcode_duration_seconds`）
#[tracing::instrument(skip(state, transcode_semaphore, task_type_semaphore), fields(task_id))]
pub async fn run_hls_worker(
    state: &crate::AppState,
    transcode_semaphore: std::sync::Arc<tokio::sync::Semaphore>,
    task_type_semaphore: Option<std::sync::Arc<tokio::sync::Semaphore>>,
) -> Result<(), AppError> {
    // 从队列抢占一条待处理任务
    let task = match state.task_queue.dequeue_pending_task("hls_preview").await? {
        Some(t) => t,
        None => return Ok(()),
    };

    tracing::Span::current().record("task_id", tracing::field::display(task.id));

    // 解析 payload
    let payload: HlsPreviewPayload = serde_json::from_value(task.payload.clone())
        .map_err(|e| AppError::File(format!("解析 hls_preview payload 失败: {}", e)))?;

    let started_at = Instant::now();

    // 仅支持本地存储；S3 等后端后续扩展
    if payload.storage_backend != "local" {
        state
            .task_queue
            .mark_failed(
                task.id,
                "hls_preview only supports local storage backend for now",
            )
            .await?;
        return Ok(());
    }

    // 查询最新文件记录，避免任务创建后文件被删除或移动
    let file: File = match state
        .file_service
        .get_file(payload.file_id, payload.user_id)
        .await
    {
        Ok(f) => f,
        Err(AppError::NotFound) => {
            state
                .task_queue
                .mark_failed(
                    task.id,
                    "hls_preview source file not found (deleted or moved)",
                )
                .await?;
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    // 获取全局并发配额与任务类型配额
    let _global_permit = transcode_semaphore
        .acquire_owned()
        .await
        .map_err(|_| AppError::Internal)?;
    let _type_permit = if let Some(sema) = task_type_semaphore {
        Some(sema.acquire_owned().await.map_err(|_| AppError::Internal)?)
    } else {
        None
    };

    // 执行 HLS 转码
    match state.file_service.ensure_hls_ready(&file).await {
        Ok(_) => {
            let elapsed = started_at.elapsed();
            // 成功指标
            counter!(
                "transcode_jobs_total",
                "task_type" => "hls_preview",
                "status" => "succeeded"
            )
            .increment(1);
            histogram!(
                "transcode_duration_seconds",
                "task_type" => "hls_preview"
            )
            .record(elapsed.as_secs_f64());

            tracing::info!(
                task_id = %task.id,
                user_id = %payload.user_id,
                file_id = %payload.file_id,
                duration_ms = elapsed.as_millis() as u64,
                "hls_preview transcode succeeded"
            );

            state.task_queue.mark_succeeded(task.id).await?;
        }
        Err(e) => {
            let elapsed = started_at.elapsed();
            let msg = format!("hls_preview transcode failed: {}", e);

            // 失败指标
            counter!(
                "transcode_jobs_total",
                "task_type" => "hls_preview",
                "status" => "failed"
            )
            .increment(1);
            histogram!(
                "transcode_duration_seconds",
                "task_type" => "hls_preview"
            )
            .record(elapsed.as_secs_f64());

            tracing::error!(
                task_id = %task.id,
                user_id = %payload.user_id,
                file_id = %payload.file_id,
                duration_ms = elapsed.as_millis() as u64,
                error = %e,
                "hls_preview transcode failed"
            );

            // 若尚未达到最大重试次数，则重新排回 pending 队列；否则标记为最终失败（死信）。
            if task.attempts < MAX_ATTEMPTS {
                state
                    .task_queue
                    .mark_pending_with_error(task.id, task.attempts, &msg)
                    .await?;
            } else {
                state.task_queue.mark_failed(task.id, &msg).await?;
            }
        }
    }

    Ok(())
}

// =============================================================================
// Worker：回收站过期清理
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashCleanupPayload {
    pub retention_days: i64,
    pub batch_limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FulltextIndexPayload {
    pub file_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FulltextRemovePayload {
    pub file_id: Uuid,
}

#[tracing::instrument(skip(state), fields(task_id))]
pub async fn run_fulltext_index_worker(state: &crate::AppState) -> Result<(), AppError> {
    let task = match state
        .task_queue
        .dequeue_pending_task("search_index_file")
        .await?
    {
        Some(task) => task,
        None => return Ok(()),
    };
    tracing::Span::current().record("task_id", tracing::field::display(task.id));
    let payload: FulltextIndexPayload = serde_json::from_value(task.payload.clone())
        .map_err(|e| AppError::File(format!("解析 search_index_file payload 失败: {}", e)))?;
    let started_at = Instant::now();

    let result = FulltextIndexer::new(state)
        .index_file(payload.file_id, payload.user_id)
        .await;

    match result {
        Ok(()) => {
            histogram!("fulltext_index_duration_seconds")
                .record(started_at.elapsed().as_secs_f64());
            state.task_queue.mark_succeeded(task.id).await?;
        }
        Err(AppError::NotFound) => {
            state
                .task_queue
                .mark_failed(task.id, "search_index_file source file not found")
                .await?;
        }
        Err(e) => {
            let msg = format!("search_index_file failed: {e}");
            if task.attempts < MAX_ATTEMPTS {
                state
                    .task_queue
                    .mark_pending_with_error(task.id, task.attempts, &msg)
                    .await?;
            } else {
                state.task_queue.mark_failed(task.id, &msg).await?;
            }
        }
    }
    Ok(())
}

#[tracing::instrument(skip(state), fields(task_id))]
pub async fn run_fulltext_remove_worker(state: &crate::AppState) -> Result<(), AppError> {
    let task = match state
        .task_queue
        .dequeue_pending_task("search_remove_file")
        .await?
    {
        Some(task) => task,
        None => return Ok(()),
    };
    tracing::Span::current().record("task_id", tracing::field::display(task.id));
    let payload: FulltextRemovePayload = serde_json::from_value(task.payload.clone())
        .map_err(|e| AppError::File(format!("解析 search_remove_file payload 失败: {}", e)))?;
    let result = state.search_index.remove_document(payload.file_id);
    match result {
        Ok(()) => state.task_queue.mark_succeeded(task.id).await?,
        Err(e) => {
            let msg = format!("search_remove_file failed: {e}");
            state.task_queue.mark_failed(task.id, &msg).await?;
        }
    }
    Ok(())
}

#[tracing::instrument(skip(state), fields(task_id))]
pub async fn run_trash_cleanup_worker(state: &crate::AppState) -> Result<(), AppError> {
    let task = match state
        .task_queue
        .dequeue_pending_task("trash_cleanup")
        .await?
    {
        Some(t) => t,
        None => return Ok(()),
    };

    tracing::Span::current().record("task_id", tracing::field::display(task.id));

    let payload: TrashCleanupPayload = serde_json::from_value(task.payload.clone())
        .map_err(|e| AppError::File(format!("解析 trash_cleanup payload 失败: {}", e)))?;
    let started_at = Instant::now();

    match state
        .file_service
        .purge_expired_trash(payload.retention_days, payload.batch_limit)
        .await
    {
        Ok(deleted) => {
            let elapsed = started_at.elapsed();
            counter!("trash_cleanup_deleted_total").increment(deleted);
            histogram!("trash_cleanup_duration_seconds").record(elapsed.as_secs_f64());
            tracing::info!(
                task_id = %task.id,
                deleted,
                retention_days = payload.retention_days,
                batch_limit = payload.batch_limit,
                duration_ms = elapsed.as_millis() as u64,
                "trash cleanup succeeded"
            );
            state.task_queue.mark_succeeded(task.id).await?;
        }
        Err(e) => {
            let msg = format!("trash_cleanup failed: {}", e);
            tracing::error!(task_id = %task.id, error = %e, "trash cleanup failed");
            if task.attempts < MAX_ATTEMPTS {
                state
                    .task_queue
                    .mark_pending_with_error(task.id, task.attempts, &msg)
                    .await?;
            } else {
                state.task_queue.mark_failed(task.id, &msg).await?;
            }
        }
    }

    Ok(())
}
