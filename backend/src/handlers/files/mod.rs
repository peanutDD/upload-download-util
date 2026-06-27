//! # Files Handlers
//!
//! `files` 相关的 HTTP handlers。
//!
//! ## 为什么要拆分
//!
//! 旧的 `handlers/files.rs` 体积过大（下载/上传/列表/批量/分块上传等都混在一起），
//! 会导致：
//! - 导航困难、改动风险高
//! - 不同业务域之间隐式耦合（共享 helper/导入/类型）
//! - 单文件编译/增量编译负担更重
//!
//! 因此这里按“功能域/业务”拆分成多个小模块，并在本 `mod.rs` 中统一对外导出，
//! **保证外部（路由层）仍然可以通过 `crate::handlers::files::*` 使用原有函数名**，
//! 从而降低重构成本。
//!
//! ## 模块划分
//!
//! - `download`: 下载/预览（含 Range、ETag、Last-Modified 等 HTTP 缓存/预条件）
//! - `upload`: 普通上传（multipart，流式落盘到临时文件）
//! - `list`: 文件列表查询（分页/过滤/搜索）
//! - `delete`: 单文件删除
//! - `batch`: 批量删除/批量移动/批量打包下载（ZIP）
//! - `storage`: 存储用量与配额
//! - `categories`: 分类列表
//! - `chunked_upload`: 分块上传（可恢复上传）
//!
//! handler 依然坚持“薄层设计”：HTTP 解析/响应构建在 handlers，业务逻辑在 `FileService`。

mod batch;
mod categories;
mod chunked_upload;
mod delete;
mod download;
mod fulltext_search;
mod instant_upload;
mod list;
mod semantic_search;
mod storage;
mod trash;
mod upload;
mod versions;
mod video;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::Json;
use serde_json::json;
use uuid::Uuid;

use crate::extractors::AuthenticatedUser;
use crate::models::file::RenameFileRequest;
use crate::utils::{json_response, AppError};
use crate::AppState;

pub use batch::{
    batch_delete_handler, batch_download_zip_handler, batch_download_zip_post_handler,
    batch_get_handler, batch_move_handler,
};
pub use categories::categories_handler;
pub use chunked_upload::{
    chunked_upload_abort_handler, chunked_upload_chunk_handler, chunked_upload_complete_handler,
    chunked_upload_init_handler, chunked_upload_status_handler,
};
pub use delete::delete_file_handler;
pub use download::{
    download_file_handler, hls_asset_handler, hls_playlist_handler, hls_prepare_handler,
    hls_status_handler, preview_file_handler, thumbnail_file_handler,
};
pub use fulltext_search::{fulltext_search_handler, ocr_status_handler};
pub use instant_upload::instant_upload_handler;
pub use list::list_files_handler;
pub use semantic_search::semantic_search_handler;
pub use storage::storage_usage_handler;
pub use trash::{
    batch_permanently_delete_files_handler, batch_restore_files_handler, empty_trash_handler,
    list_trash_handler, permanently_delete_file_handler, restore_file_handler,
};
pub use upload::upload_file_handler;
pub use versions::{
    delete_version_handler, get_file_version_handler, list_file_versions_handler,
    restore_version_handler, update_version_label_handler,
};
pub use video::{
    gif_video_preview_handler, video_preview_prepare_handler, video_preview_status_handler,
};

pub async fn rename_file_handler(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(file_id): Path<Uuid>,
    Json(req): Json<RenameFileRequest>,
) -> Result<Response, AppError> {
    let file = state
        .file_service
        .rename_file(user_id, file_id, req)
        .await?;
    if let Some(pool) = &state.redis {
        let _ = crate::services::redis::RedisService::new(pool.clone())
            .bump_user_cache_version(user_id)
            .await;
    }
    Ok(json_response(json!({ "file": file })))
}
