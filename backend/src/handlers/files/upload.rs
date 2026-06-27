//! 普通上传（multipart/form-data）
//!
//! 目标：
//! - 以流式方式读取 multipart，写入临时文件，避免大文件一次性进内存
//! - 超过配置最大文件大小时尽早熔断

use axum::extract::{Multipart, State};
use axum::response::Response;
use serde_json::json;
use uuid::Uuid;

use crate::constants::DISK_RESERVE_UPLOAD;
use crate::extractors::AuthenticatedUser;
use crate::services::file::CreateFileFromPathInput;
use crate::utils::{effective_file_mime_type, json_response, AppError};
use crate::AppState;

/// 上传文件（普通上传）
///
/// 处理 multipart/form-data 格式的文件上传请求。
///
/// # 请求格式
/// - Content-Type: `multipart/form-data`
/// - Field name: `file`
///
/// # 响应
/// ```json
/// {
///   "file": {
///     "id": "...",
///     "filename": "...",
///     ...
///   }
/// }
/// ```
pub async fn upload_file_handler(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    use tokio::io::AsyncWriteExt;

    // -------------------------------------------------------------------------
    // 解析 multipart
    // -------------------------------------------------------------------------
    //
    // 将 `file` 字段流式写入磁盘，避免大文件进入内存导致 OOM。
    // 解析 multipart 表单：以流式方式写入临时文件，避免把整个文件载入内存
    let mut file_meta: Option<(String, String, u64, std::path::PathBuf)> = None;
    let mut folder_id: Option<Uuid> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::File(format!("Failed to parse multipart: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        if name == "folder_id" {
            let value = field
                .text()
                .await
                .map_err(|e| AppError::File(format!("Failed to read folder_id: {}", e)))?;
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                folder_id = Some(
                    Uuid::parse_str(trimmed)
                        .map_err(|_| AppError::Validation("无效的 folder_id".to_string()))?,
                );
            }
            continue;
        }

        if name == "file" {
            if file_meta.is_some() {
                continue;
            }
            let filename = field
                .file_name()
                .ok_or_else(|| AppError::File("Missing filename".to_string()))?
                .to_string();

            let mime_type = effective_file_mime_type(&filename, field.content_type());

            // -----------------------------------------------------------------
            // 临时文件
            // -----------------------------------------------------------------
            //
            // 先落到临时目录；真正写入存储后端（本地/S3 等）由 FileService 统一处理。
            // 临时文件路径
            let tmp_dir = std::env::temp_dir().join("file-storage-backend");
            tokio::fs::create_dir_all(&tmp_dir)
                .await
                .map_err(|e| AppError::Storage(format!("Failed to create temp dir: {}", e)))?;
            let tmp_path = tmp_dir.join(format!("upload_{}", Uuid::new_v4()));

            // 磁盘空间保护（best-effort）
            if let Ok(free) = fs2::available_space(&tmp_dir) {
                if free.saturating_sub(DISK_RESERVE_UPLOAD) == 0 {
                    tracing::error!(
                        error_type = "disk_full",
                        tmp_dir = %tmp_dir.display(),
                        free_bytes = free,
                        reserve_bytes = DISK_RESERVE_UPLOAD,
                        "insufficient disk space for upload temp file"
                    );
                    return Err(AppError::Storage("磁盘空间不足，请稍后重试".to_string()));
                }
            }

            let mut out = tokio::fs::File::create(&tmp_path)
                .await
                .map_err(|e| AppError::Storage(format!("Failed to create temp file: {}", e)))?;

            // -----------------------------------------------------------------
            // 流式写入 + 提前熔断
            // -----------------------------------------------------------------
            //
            // 一旦超过 MAX_FILE_SIZE 立刻停止，避免继续读写造成额外 IO 与磁盘占用。
            // 流式写入并统计大小
            let mut file_size: u64 = 0;
            let mut field = field;
            while let Some(chunk) = field
                .chunk()
                .await
                .map_err(|e| AppError::File(format!("Failed to read upload chunk: {}", e)))?
            {
                file_size = file_size.saturating_add(chunk.len() as u64);

                // 早期熔断：超过配置最大值立刻停止（避免继续读入/写盘）
                if file_size > state.config.storage.max_file_size {
                    let _ = tokio::fs::remove_file(&tmp_path).await;
                    return Err(AppError::PayloadTooLarge(format!(
                        "文件大小超过限制（{} bytes）",
                        state.config.storage.max_file_size
                    )));
                }

                out.write_all(&chunk)
                    .await
                    .map_err(|e| AppError::Storage(format!("Failed to write temp file: {}", e)))?;
            }
            out.flush()
                .await
                .map_err(|e| AppError::Storage(format!("Failed to flush temp file: {}", e)))?;

            file_meta = Some((filename, mime_type, file_size, tmp_path));
        }
    }

    let (original_filename, mime_type, file_size, tmp_path) =
        file_meta.ok_or_else(|| AppError::File("No file provided".to_string()))?;

    // -------------------------------------------------------------------------
    // 内容摘要（用于秒传/去重）
    // -------------------------------------------------------------------------
    let content_sha256 = tokio::task::spawn_blocking({
        let p = tmp_path.clone();
        move || crate::utils::sha256_file_hex(&p).ok()
    })
    .await
    .ok()
    .flatten();

    let file = match state
        .file_service
        .create_file_from_path(CreateFileFromPathInput {
            user_id,
            original_filename,
            mime_type,
            file_size,
            source_path: &tmp_path,
            content_sha256: content_sha256.as_deref(),
            folder_id,
        })
        .await
    {
        Ok(file) => file,
        Err(e) => {
            // best-effort 清理：失败时不应留下临时文件占空间（即便删除失败也不阻塞返回）。
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(e);
        }
    };

    // -------------------------------------------------------------------------
    // 成功后的附带动作
    // -------------------------------------------------------------------------
    // 记录上传文件体积指标（成功路径）
    crate::middleware::metrics::record_file_operation("upload", file_size, true);

    if let Some(pool) = &state.redis {
        // 成功写入后，失效该用户的读缓存（文件列表/分类/用量等）。
        let _ = crate::services::redis::RedisService::new(pool.clone())
            .bump_user_cache_version(user_id)
            .await;
    }

    Ok(json_response(json!({ "file": file })))
}
