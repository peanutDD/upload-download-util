//! 上传相关（普通上传 / 从本地路径上传）

use std::path::Path;
use std::sync::Arc;

use uuid::Uuid;

use crate::models::file::File;
use crate::models::file::FileResponse;
use crate::services::embeddings::EmbeddingService;
use crate::services::file_content_extractor::FileContentExtractor;
use crate::services::storage::StorageBackend;
use crate::utils::AppError;
use sqlx::PgPool;

use super::FileService;

// =============================================================================
// 输入结构
// =============================================================================
pub(crate) struct CreateFileFromPathInput<'a> {
    pub user_id: Uuid,
    pub original_filename: String,
    pub mime_type: String,
    pub file_size: u64,
    pub source_path: &'a Path,
    pub content_sha256: Option<&'a str>,
    pub folder_id: Option<Uuid>,
}

// =============================================================================
// 后台任务：向量嵌入
// =============================================================================
pub(crate) struct EmbeddingTaskInput {
    pub embedding_service: Arc<EmbeddingService>,
    pub storage: Arc<dyn StorageBackend>,
    pub file: File,
    pub mime_type: String,
    pub original_filename: String,
    pub file_id: Uuid,
    pub user_id: Uuid,
    pub pool: PgPool,
}

// =============================================================================
// 文件名策略
// =============================================================================
//
// 存储侧文件名与“展示文件名”分离：
// - 展示名（original_filename）可重复、可包含特殊字符
// - 存储名用于落盘/对象存储 key，需可控且尽量避免冲突
pub(crate) fn build_storage_filename(
    file_id: Uuid,
    original_filename: &str,
) -> Result<String, AppError> {
    let sanitized_filename = crate::utils::validation::sanitize_filename(original_filename)?;
    Ok(format!("{}_{}", file_id, sanitized_filename))
}

impl FileService {
    pub(crate) async fn enqueue_fulltext_index_task(
        &self,
        file_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        if !self.config.search.fulltext_search_enabled {
            return Ok(());
        }
        let payload = serde_json::json!({
            "file_id": file_id,
            "user_id": user_id,
        });
        let queue = crate::services::task_queue::TaskQueue::new(Arc::new(self.pool.clone()));
        queue
            .enqueue_task(
                "search_index_file",
                payload,
                Some(&format!("search:{file_id}")),
            )
            .await?;
        Ok(())
    }

    pub(crate) async fn enqueue_fulltext_remove_task(&self, file_id: Uuid) -> Result<(), AppError> {
        if !self.config.search.fulltext_search_enabled {
            return Ok(());
        }
        let payload = serde_json::json!({ "file_id": file_id });
        let queue = crate::services::task_queue::TaskQueue::new(Arc::new(self.pool.clone()));
        queue
            .enqueue_task(
                "search_remove_file",
                payload,
                Some(&format!("search:{file_id}")),
            )
            .await?;
        Ok(())
    }

    pub(crate) async fn enqueue_fulltext_index_task_best_effort(
        &self,
        file_id: Uuid,
        user_id: Uuid,
    ) {
        if let Err(error) = self.enqueue_fulltext_index_task(file_id, user_id).await {
            tracing::warn!(%file_id, %user_id, %error, "failed to enqueue fulltext index task");
        }
    }

    pub(crate) async fn enqueue_fulltext_remove_task_best_effort(&self, file_id: Uuid) {
        if let Err(error) = self.enqueue_fulltext_remove_task(file_id).await {
            tracing::warn!(%file_id, %error, "failed to enqueue fulltext remove task");
        }
    }

    // =============================================================================
    // 创建文件（bytes）
    // =============================================================================
    /// 从内存数据创建文件。
    ///
    /// 当前 HTTP 上传路径统一走 multipart / 分片上传，因此暂未直接使用该方法；
    /// 预留给「纯 JSON + Base64 / 直接从第三方拉取字节流再入库」这类 API。
    /// 当你需要提供「服务端从远程拉取并写入存储」的能力时，可以复用此逻辑。
    #[allow(dead_code)]
    pub async fn create_file(
        &self,
        user_id: Uuid,
        original_filename: String,
        mime_type: String,
        file_size: u64,
        data: Vec<u8>,
    ) -> Result<FileResponse, AppError> {
        self.ensure_can_store_detailed(user_id, &mime_type, file_size)
            .await?;

        let file_id = Uuid::new_v4();
        let storage_filename = build_storage_filename(file_id, &original_filename)?;

        // 写入存储后端（本地/S3 等）
        let file_path = self
            .storage
            .save_file(user_id, file_id, &storage_filename, &data)
            .await?;

        let inserted = self
            .files_repo
            .insert(
                file_id,
                user_id,
                &storage_filename,
                &original_filename,
                &file_path,
                file_size,
                &mime_type,
                &self.config.storage.backend,
                None,
                None,
            )
            .await
            .map(FileResponse::from);

        // 若落库失败，尽量清理已写入的存储文件，避免产生"孤儿文件"占用空间
        if inserted.is_err() {
            let _ = self.storage.delete_file(&file_path).await;
        }

        inserted
    }

    // =============================================================================
    // 创建文件（本地路径 -> 存储 + 数据库）
    // =============================================================================
    /// 从本地路径创建文件，可选传入已计算的内容 SHA256（用于秒传落库）
    ///
    /// 如果已存在同名文件（在同一文件夹下），会自动创建版本：
    /// 1. 将当前文件保存为新版本
    /// 2. 将旧文件保存为历史版本
    /// 3. 清理超过保留数量的旧版本（只保留最近2个）
    pub(crate) async fn create_file_from_path(
        &self,
        input: CreateFileFromPathInput<'_>,
    ) -> Result<FileResponse, AppError> {
        let CreateFileFromPathInput {
            user_id,
            original_filename,
            mime_type,
            file_size,
            source_path,
            content_sha256,
            folder_id,
        } = input;
        self.ensure_can_store_detailed(user_id, &mime_type, file_size)
            .await?;
        self.ensure_folder_belongs_to_user(user_id, folder_id)
            .await?;

        // 同目录同名文件：用“版本化覆盖”保证用户体验（同名上传即更新），同时保留历史版本回滚能力。
        // 检查是否存在同名文件（在同一文件夹下）
        let existing_file = self
            .files_repo
            .find_by_name_and_folder(user_id, &original_filename, folder_id)
            .await?;

        let file_id = if let Some(existing) = existing_file {
            // 存在同名文件，需要创建版本
            let existing_file_id = existing.id;

            // 版本号递增：以 files.id 为聚合根，file_versions 记录历史快照。
            // 获取当前最大版本号（如果没有版本，max_version = 0）
            let max_version = self
                .file_versions_repo
                .get_max_version_number(existing_file_id)
                .await?;
            let next_version = max_version + 1;

            // 将旧文件保存为历史版本（版本号 = next_version）
            let _ = self
                .file_versions_repo
                .create_version(
                    existing_file_id,
                    user_id,
                    next_version,
                    &existing.filename,
                    &existing.original_filename,
                    &existing.file_path,
                    existing.file_size as u64,
                    &existing.mime_type,
                    &existing.storage_backend,
                    existing.content_sha256.as_deref(),
                )
                .await;

            // 新内容写入存储后端：使用 existing_file_id 复用同一个 files 记录。
            // 将新文件保存为当前文件（更新 files 表）
            let storage_filename = build_storage_filename(existing_file_id, &original_filename)?;
            let file_path = self
                .storage
                .save_file_from_path(user_id, existing_file_id, &storage_filename, source_path)
                .await?;

            // 更新文件记录
            sqlx::query(
                "UPDATE files SET 
                    filename = $1, file_path = $2, file_size = $3, 
                    mime_type = $4, content_sha256 = $5, updated_at = CURRENT_TIMESTAMP
                 WHERE id = $6 AND user_id = $7",
            )
            .bind(&storage_filename)
            .bind(&file_path)
            .bind(file_size as i64)
            .bind(&mime_type)
            .bind(content_sha256)
            .bind(existing_file_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

            // 清理旧版本（只保留最近2个）
            let old_version_paths = self
                .file_versions_repo
                .cleanup_old_versions(existing_file_id, 2)
                .await?;
            for old_path in old_version_paths {
                let _ = self.storage.delete_file(&old_path).await;
            }

            // 重新查询更新后的文件
            let file_response = self
                .files_repo
                .find_by_id(existing_file_id, user_id)
                .await?
                .ok_or(AppError::NotFound)
                .map(FileResponse::from)?;

            // 如果配置了嵌入服务，异步提取内容并生成向量嵌入（不阻塞上传流程）
            if let Some(embedding_service) = &self.embedding_service {
                let file_clone = self
                    .files_repo
                    .find_by_id(existing_file_id, user_id)
                    .await?
                    .ok_or(AppError::NotFound)?;
                let task = EmbeddingTaskInput {
                    embedding_service: embedding_service.clone(),
                    storage: self.storage.clone(),
                    file: file_clone,
                    mime_type: mime_type.clone(),
                    original_filename: original_filename.clone(),
                    file_id: file_response.id,
                    user_id,
                    pool: self.pool.clone(),
                };

                tokio::spawn(async move {
                    Self::generate_embedding_with_content(task).await;
                });
            }

            self.enqueue_fulltext_index_task_best_effort(file_response.id, user_id)
                .await;
            file_response
        } else {
            // 不存在同名文件，创建新文件
            let new_file_id = Uuid::new_v4();
            let storage_filename = build_storage_filename(new_file_id, &original_filename)?;

            // 从本地路径写入存储后端，避免把文件整体读入内存
            let file_path = self
                .storage
                .save_file_from_path(user_id, new_file_id, &storage_filename, source_path)
                .await?;

            let inserted = self
                .files_repo
                .insert(
                    new_file_id,
                    user_id,
                    &storage_filename,
                    &original_filename,
                    &file_path,
                    file_size,
                    &mime_type,
                    &self.config.storage.backend,
                    content_sha256,
                    folder_id,
                )
                .await
                .map(FileResponse::from);

            // 若落库失败，尽量清理已写入的存储文件（此时 source_path 可能已被 move/删除）
            if inserted.is_err() {
                let _ = self.storage.delete_file(&file_path).await;
            }

            let file_response = inserted?;

            // 如果配置了嵌入服务，异步提取内容并生成向量嵌入（不阻塞上传流程）
            if let Some(embedding_service) = &self.embedding_service {
                let file_clone = self
                    .files_repo
                    .find_by_id(file_response.id, user_id)
                    .await?
                    .ok_or(AppError::NotFound)?;
                let task = EmbeddingTaskInput {
                    embedding_service: embedding_service.clone(),
                    storage: self.storage.clone(),
                    file: file_clone,
                    mime_type: mime_type.clone(),
                    original_filename: original_filename.clone(),
                    file_id: file_response.id,
                    user_id,
                    pool: self.pool.clone(),
                };

                tokio::spawn(async move {
                    Self::generate_embedding_with_content(task).await;
                });
            }

            self.enqueue_fulltext_index_task_best_effort(file_response.id, user_id)
                .await;
            file_response
        };

        Ok(file_id)
    }

    // =============================================================================
    // 向量嵌入（异步）
    // =============================================================================
    /// 异步生成文件向量嵌入（包含文件名和内容）
    ///
    /// 这是一个辅助函数，用于在后台任务中提取文件内容并生成向量
    pub(crate) async fn generate_embedding_with_content(task: EmbeddingTaskInput) {
        let EmbeddingTaskInput {
            embedding_service,
            storage,
            file,
            mime_type,
            original_filename,
            file_id,
            user_id,
            pool,
        } = task;
        // 1. 读取文件内容
        let file_data = match storage.get_file(&file.file_path).await {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!(
                    "Failed to read file {} for content extraction: {}",
                    file_id,
                    e
                );
                return;
            }
        };

        // 2. 提取文本内容
        let content =
            match FileContentExtractor::extract_text(&file_data, &mime_type, &original_filename) {
                Ok(text) => {
                    if text.trim().is_empty() {
                        tracing::debug!("No text content extracted from file {}", file_id);
                        // 即使没有内容，也使用文件名生成向量
                        format!("文件名: {}", original_filename)
                    } else {
                        text
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to extract content from file {}: {}", file_id, e);
                    // 提取失败时，至少使用文件名
                    format!("文件名: {}", original_filename)
                }
            };

        // 3. 组合文件名和内容用于生成向量
        let search_text = FileContentExtractor::combine_for_embedding(&original_filename, &content);

        // 4. 生成向量嵌入
        match embedding_service.generate_embedding(&search_text).await {
            Ok(embedding) => {
                use crate::services::file::SemanticSearchService;
                let search_service = SemanticSearchService::new(pool);
                if let Err(e) = search_service
                    .update_file_embedding(file_id, user_id, &embedding)
                    .await
                {
                    tracing::warn!("Failed to update file embedding for {}: {}", file_id, e);
                } else {
                    tracing::debug!(
                        "Successfully generated embedding for file {} (with content)",
                        file_id
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Failed to generate embedding for file {}: {}", file_id, e);
            }
        }
    }
}
