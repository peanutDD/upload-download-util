//! 下载与预览（含 Range、缓存与预条件）
//!
//! 入口只做编排：解析请求头 -> 预条件/缓存判定 -> Range 判定 -> 交给 GET/HEAD 子模块构造响应。
//! 具体实现拆分到：
//! - `headers`：实体头/缓存头/Range 相关头复用
//! - `ranges`：Range 解析与规范化
//! - `responses`：304/412/416 等常用响应
//! - `head`：HEAD 响应构造
//! - `get`：GET 响应构造（含 body 传输）
//! - `multipart`：多段 Range（multipart/byteranges）流式 body

mod get;
mod head;
mod headers;
mod multipart;
mod ranges;
mod responses;

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use deadpool_redis::redis::cmd;
use serde_json::json;
use uuid::Uuid;

use crate::extractors::AuthenticatedUserQuery;
use crate::utils::hls_processing_response;
use crate::utils::response::file_response;
use crate::utils::thumbnail::generate_thumbnail_webp;
use crate::utils::{effective_file_mime_type, AppError};
use crate::AppState;

use headers::EntityHeaders;
use ranges::parse_ranges;
use responses::{not_modified_response, precondition_failed_response};

use crate::constants::CACHE_CONTROL_THUMBNAIL;

fn compute_etag(file_id: Uuid, updated_at_unix: i64, total_size: u64) -> String {
    // Weak ETag：用于缓存/断点续传的实用型标识（不保证字节级强一致）
    format!("W/\"{}-{}-{}\"", file_id, updated_at_unix, total_size)
}

fn is_gif_mime(mime_type: &str) -> bool {
    mime_type.to_lowercase().starts_with("image/gif")
}

fn filename_is_gif(name: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("gif"))
        .unwrap_or(false)
}

fn resolve_effective_mime(file: &crate::models::file::File) -> String {
    if is_gif_mime(&file.mime_type) || filename_is_gif(&file.original_filename) {
        return "image/gif".to_string();
    }
    effective_file_mime_type(&file.original_filename, Some(&file.mime_type))
}

async fn file_get_or_head_response(
    state: &AppState,
    user_id: Uuid,
    method: Method,
    headers: HeaderMap,
    file_id: Uuid,
    inline: bool,
) -> Result<Response, AppError> {
    use httpdate::{fmt_http_date, parse_http_date};
    use std::time::{Duration, SystemTime};

    let file = state.file_service.get_file(file_id, user_id).await?;

    if state.config.storage.download_mode != "proxy" {
        let cd_type = if inline { "inline" } else { "attachment" };
        let filename = file.original_filename.replace('"', "");
        let content_disposition = format!("{}; filename=\"{}\"", cd_type, filename);
        let url = state
            .storage
            .presign_download_url(
                &file.file_path,
                state.config.storage.presign_ttl_secs,
                Some(&file.mime_type),
                Some(&content_disposition),
            )
            .await?
            .ok_or(AppError::Internal)?;

        let res = Response::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, url)
            .body(axum::body::Body::empty())
            .map_err(|_| AppError::Internal)?;
        return Ok(res);
    }

    let total_size = file.file_size.max(0) as u64;
    let effective_mime = resolve_effective_mime(&file);
    let etag_str = compute_etag(file.id, file.updated_at.timestamp(), total_size);
    let last_modified_str = {
        let ts = file.updated_at.timestamp().max(0) as u64;
        fmt_http_date(SystemTime::UNIX_EPOCH + Duration::from_secs(ts))
    };
    let entity_headers = EntityHeaders::new(etag_str, last_modified_str)?;

    // headers
    let range_header = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
    let if_none_match = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok());
    let if_range = headers.get(header::IF_RANGE).and_then(|v| v.to_str().ok());
    let if_match = headers.get(header::IF_MATCH).and_then(|v| v.to_str().ok());
    let if_unmodified_since = headers
        .get(header::IF_UNMODIFIED_SINCE)
        .and_then(|v| v.to_str().ok());
    let if_modified_since = headers
        .get(header::IF_MODIFIED_SINCE)
        .and_then(|v| v.to_str().ok());

    // 预条件：If-Match（不匹配则 412）
    if let Some(v) = if_match {
        if v != "*" && v != entity_headers.etag_str {
            return Ok(precondition_failed_response(&entity_headers));
        }
    }

    // 预条件：If-Unmodified-Since（已被修改则 412）
    if let Some(v) = if_unmodified_since {
        if let Ok(t) = parse_http_date(v) {
            let updated_ts = file.updated_at.timestamp().max(0) as u64;
            let t_ts = t
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if updated_ts > t_ts {
                return Ok(precondition_failed_response(&entity_headers));
            }
        }
    }

    // If-None-Match（仅在非 Range 情况下返回 304）
    // 但在返回 304 之前，先验证文件是否真实存在且非空
    if range_header.is_none() && if_none_match == Some(entity_headers.etag_str.as_str()) {
        // 验证文件是否存在，防止数据库记录存在但文件缺失的情况
        if state.file_service.verify_file_exists(&file).await.is_ok() {
            tracing::info!(
                file_id = %file_id,
                etag = %entity_headers.etag_str,
                inline = inline,
                "preview/download 304 Not Modified (If-None-Match match)"
            );
            return Ok(not_modified_response(&entity_headers, inline));
        }
        // 文件不存在，继续处理以返回错误或重新生成内容
    }

    // If-Modified-Since（仅在非 Range 情况下生效；Range 推荐用 If-Range）
    if range_header.is_none() {
        if let Some(v) = if_modified_since {
            if let Ok(t) = parse_http_date(v) {
                let updated_ts = file.updated_at.timestamp().max(0) as u64;
                let t_ts = t
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                if updated_ts <= t_ts {
                    // 同样验证文件是否存在
                    if state.file_service.verify_file_exists(&file).await.is_ok() {
                        tracing::info!(
                            file_id = %file_id,
                            last_modified = %entity_headers
                                .last_modified_header()
                                .to_str()
                                .unwrap_or(""),
                            inline = inline,
                            "preview/download 304 Not Modified (If-Modified-Since match)"
                        );
                        return Ok(not_modified_response(&entity_headers, inline));
                    }
                }
            }
        }
    }

    // Range 解析（支持多段）
    let mut ranges = parse_ranges(range_header, total_size)?;

    // If-Range：不匹配则忽略 Range，改走全量
    //
    // 兼容两种 If-Range 语义：
    // - ETag：完全匹配才允许 Range
    // - HTTP-date：资源“未在该时间之后修改”才允许 Range
    if ranges.as_ref().map(|r| !r.is_empty()).unwrap_or(false) {
        if let Some(if_range) = if_range {
            if if_range == entity_headers.etag_str {
                // ok: ETag match
            } else if let Ok(t) = parse_http_date(if_range) {
                let updated_ts = file.updated_at.timestamp().max(0) as u64;
                let updated_time = SystemTime::UNIX_EPOCH + Duration::from_secs(updated_ts);
                if updated_time > t {
                    ranges = None;
                }
            } else {
                ranges = None;
            }
        }
    }

    if method == Method::HEAD {
        return head::build_head_response(
            &file.original_filename,
            &effective_mime,
            inline,
            total_size,
            ranges,
            &entity_headers,
        );
    }

    get::build_get_response(
        state,
        &file,
        &effective_mime,
        inline,
        total_size,
        ranges,
        &entity_headers,
    )
    .await
}

/// 下载文件
///
/// 返回文件内容，设置 `Content-Disposition: attachment` 触发下载。
pub async fn download_file_handler(
    State(state): State<AppState>,
    AuthenticatedUserQuery(user_id): AuthenticatedUserQuery,
    method: Method,
    headers: HeaderMap,
    Path(file_id): Path<Uuid>,
) -> Result<Response, AppError> {
    file_get_or_head_response(&state, user_id, method, headers, file_id, false).await
}

/// 预览文件
///
/// 返回文件内容，设置 `Content-Disposition: inline` 在浏览器中显示。
pub async fn preview_file_handler(
    State(state): State<AppState>,
    AuthenticatedUserQuery(user_id): AuthenticatedUserQuery,
    method: Method,
    headers: HeaderMap,
    Path(file_id): Path<Uuid>,
) -> Result<Response, AppError> {
    file_get_or_head_response(&state, user_id, method, headers, file_id, true).await
}

/// 缩略图查询参数：最大边长（像素），默认 400，范围 64..=800
#[derive(serde::Deserialize)]
pub struct ThumbnailQuery {
    #[serde(default = "default_thumb_size")]
    pub w: u32,
}

fn default_thumb_size() -> u32 {
    400
}

/// 图片缩略图（方案 B：先读盘，无则生成并写盘；GIF 只解第一帧）
///
/// 仅支持 `image/*`，返回 JPEG 缩略图。
/// 支持 ETag 条件请求，命中缓存时返回 304。
pub async fn thumbnail_file_handler(
    State(state): State<AppState>,
    AuthenticatedUserQuery(user_id): AuthenticatedUserQuery,
    Path(file_id): Path<Uuid>,
    Query(q): Query<ThumbnailQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    // -------------------------------------------------------------------------
    // 缩略图：磁盘缓存 + 可选 Redis 锁（防击穿）
    // -------------------------------------------------------------------------
    //
    // 为什么需要锁：
    // - 缩略图生成属于 CPU 密集（解码/缩放/编码）
    // - 多实例情况下同一文件的缩略图请求可能并发打到不同副本
    // - 更偏好“一个实例生成，其它实例短暂等待磁盘缓存出现”
    let file = state
        .file_service
        .get_file_for_thumbnail(file_id, user_id)
        .await?;

    let is_supported = file.mime_type.starts_with("image/");
    if !is_supported {
        return Err(AppError::NotFound);
    }

    let w = q.w.clamp(64, 800);

    // 计算缩略图的 ETag（基于 file_id + updated_at + width）
    let thumbnail_etag = format!(
        "W/\"thumb-{}-{}-{}\"",
        file_id,
        file.updated_at.timestamp(),
        w
    );
    let etag_header = HeaderValue::from_str(&thumbnail_etag).map_err(|_| AppError::Internal)?;

    // 检查 If-None-Match 条件请求
    if let Some(if_none_match) = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
    {
        if if_none_match == thumbnail_etag.as_str() {
            // 检查缩略图是否存在（磁盘缓存）
            if state
                .file_service
                .get_thumbnail(file_id, user_id)
                .await
                .is_ok()
            {
                tracing::info!(
                    file_id = %file_id,
                    width = w,
                    etag = %thumbnail_etag,
                    "thumbnail 304 Not Modified (ETag match)"
                );
                let mut res = StatusCode::NOT_MODIFIED.into_response();
                res.headers_mut().insert(header::ETAG, etag_header);
                res.headers_mut().insert(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static(CACHE_CONTROL_THUMBNAIL),
                );
                return Ok(res);
            }
        }
    }

    // 方案 B：先读已存在的缩略图（按用户隔离）
    if let Ok(cached) = state.file_service.get_thumbnail(file_id, user_id).await {
        tracing::info!(
            file_id = %file_id,
            width = w,
            size_bytes = cached.len(),
            "thumbnail served from disk cache (200 OK)"
        );
        let mut res = file_response(cached, "thumb.webp", "image/webp", true)
            .map_err(|_| AppError::Internal)?;
        res.headers_mut().insert(header::ETAG, etag_header);
        res.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static(CACHE_CONTROL_THUMBNAIL),
        );
        return Ok(res);
    }

    let mut thumb_lock_key: Option<String> = None;
    if let Some(pool) = &state.redis {
        if let Ok(mut conn) = pool.get().await {
            let lock_key = format!("lock:thumb:{}:{}", file_id, w);
            let acquired: Result<Option<String>, _> = cmd("SET")
                .arg(&lock_key)
                .arg("1")
                .arg("NX")
                .arg("PX")
                .arg(30000)
                .query_async(&mut conn)
                .await;

            if acquired.ok().flatten().is_some() {
                thumb_lock_key = Some(lock_key.clone());
            } else {
                // 其他实例正在生成：短暂等待磁盘缓存出现，避免重复生成。
                for _ in 0..10 {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    if let Ok(cached) = state.file_service.get_thumbnail(file_id, user_id).await {
                        let mut res = file_response(cached, "thumb.webp", "image/webp", true)
                            .map_err(|_| AppError::Internal)?;
                        res.headers_mut().insert(header::ETAG, etag_header);
                        res.headers_mut().insert(
                            header::CACHE_CONTROL,
                            HeaderValue::from_static(CACHE_CONTROL_THUMBNAIL),
                        );
                        return Ok(res);
                    }
                }

                // 仍未就绪：再尝试抢一次锁，减少长尾等待导致的重复请求放大。
                let acquired_again: Result<Option<String>, _> = cmd("SET")
                    .arg(&lock_key)
                    .arg("1")
                    .arg("NX")
                    .arg("PX")
                    .arg(30000)
                    .query_async(&mut conn)
                    .await;
                if acquired_again.ok().flatten().is_some() {
                    thumb_lock_key = Some(lock_key.clone());
                }
            }
        }
    }

    // 无缓存：在阻塞线程中生成缩略图，避免长时间占用 async 工作线程导致超时或无法正确返回
    tracing::info!(
        file_id = %file_id,
        width = w,
        "thumbnail not cached, generating new thumbnail"
    );
    let data = state.file_service.get_file_data(&file).await?;
    let mime_type = file.mime_type.clone();
    let buf = tokio::task::spawn_blocking(move || generate_thumbnail_webp(data, mime_type, w))
        .await
        .map_err(|_| AppError::Internal)?;
    let buf = match buf {
        Ok(b) => b,
        Err(AppError::File(e)) => {
            tracing::info!(file_id = %file_id, error = %e, "缩略图生成失败，返回 404");
            return Err(AppError::NotFound);
        }
        Err(e) => return Err(e),
    };

    if let Err(e) = state
        .file_service
        .save_thumbnail(file_id, user_id, &buf)
        .await
    {
        tracing::warn!(file_id = %file_id, error = %e, "缩略图生成成功但保存失败，下次请求将重新生成");
    }

    // best-effort 提前释放：生成成功后立即释放锁，缩短其它请求等待时间；
    // 同时保留 TTL 作为兜底，防止进程崩溃导致锁永久占用。
    if let (Some(pool), Some(lock_key)) = (&state.redis, thumb_lock_key.as_ref()) {
        if let Ok(mut conn) = pool.get().await {
            let _: Result<i32, _> = cmd("DEL").arg(lock_key).query_async(&mut conn).await;
        }
    }

    tracing::info!(
        file_id = %file_id,
        width = w,
        size_bytes = buf.len(),
        "thumbnail generated and saved (200 OK)"
    );
    let mut res =
        file_response(buf, "thumb.webp", "image/webp", true).map_err(|_| AppError::Internal)?;
    res.headers_mut().insert(header::ETAG, etag_header);
    res.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(CACHE_CONTROL_THUMBNAIL),
    );
    Ok(res)
}

pub async fn hls_prepare_handler(
    State(state): State<AppState>,
    AuthenticatedUserQuery(user_id): AuthenticatedUserQuery,
    Path(file_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let file = state.file_service.get_file(file_id, user_id).await?;
    if !state.file_service.should_use_hls(&file).await || file.storage_backend != "local" {
        return Ok(Json(json!({ "status": "unsupported" })));
    }
    let out_dir = state.file_service.hls_output_dir(file_id);
    let playlist_path = out_dir.join("playlist.m3u8");
    let master_path = out_dir.join("master.m3u8");
    if tokio::fs::try_exists(&playlist_path)
        .await
        .map_err(|e| AppError::File(e.to_string()))?
        || tokio::fs::try_exists(&master_path)
            .await
            .map_err(|e| AppError::File(e.to_string()))?
    {
        return Ok(Json(json!({ "status": "ready" })));
    }

    // 使用任务队列排队（幂等：同 file_id 的 pending/running 任务自动复用）
    // 相比 tokio::spawn 的优势：
    //   - 进程重启后任务可恢复（持久化到 Postgres）
    //   - 有指数退避重试 + backoff（最多 MAX_ATTEMPTS 次）
    //   - 有 Prometheus 指标（transcode_jobs_total / transcode_duration_seconds）
    //   - 并发受全局 transcode_semaphore + task_type_semaphore 保护
    let payload = serde_json::json!({
        "file_id": file.id,
        "user_id": user_id,
        "storage_backend": file.storage_backend,
    });
    let _task = state
        .task_queue
        .enqueue_task("hls_preview", payload, Some(&file.id.to_string()))
        .await?;

    Ok(Json(json!({ "status": "processing" })))
}

pub async fn hls_status_handler(
    State(state): State<AppState>,
    AuthenticatedUserQuery(user_id): AuthenticatedUserQuery,
    Path(file_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let file = state.file_service.get_file(file_id, user_id).await?;
    if !state.file_service.should_use_hls(&file).await || file.storage_backend != "local" {
        return Ok(Json(json!({ "status": "unsupported" })));
    }
    let out_dir = state.file_service.hls_output_dir(file_id);
    let playlist_path = out_dir.join("playlist.m3u8");
    let master_path = out_dir.join("master.m3u8");

    // 文件已存在：直接 ready（无需查队列）
    if tokio::fs::try_exists(&playlist_path)
        .await
        .map_err(|e| AppError::File(e.to_string()))?
        || tokio::fs::try_exists(&master_path)
            .await
            .map_err(|e| AppError::File(e.to_string()))?
    {
        return Ok(Json(json!({ "status": "ready" })));
    }

    // 查询任务队列，感知 FFmpeg 转码失败。
    // 原实现只检查文件存在性，FFmpeg 失败后会永远返回 "processing"，客户端无法感知。
    // 与 gif 的 video_preview_status_handler 保持一致。
    let latest: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT status, last_error
         FROM background_tasks
         WHERE task_type = 'hls_preview'
           AND dedupe_key = $1
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(file.id.to_string())
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::from)?;

    if let Some((status, last_error)) = latest {
        if status == "failed" {
            return Ok(Json(json!({ "status": "failed", "error": last_error })));
        }
    }

    Ok(Json(json!({ "status": "processing" })))
}

/// 返回 HLS 主列表（playlist.m3u8），供前端 hls.js 加载。仅对超过阈值的视频生效。
/// 将 m3u8 内的 segment*.ts 引用重写为 hls/segment*.ts，使相对 URL 解析到 /api/files/:id/hls/segment*.ts。
///
/// 职责边界：仅服务已就绪的 HLS 产物，不触发转码。
/// 转码由 `hls_prepare_handler` 排队 + worker 异步执行。
///
/// 为什么转码不在这里触发：
/// - playlist 接口会被 hls.js 频繁轮询，每次都可能 spawn 任务会造成重复转码
/// - 原实现中最长 30s（60×500ms）的 blocking 等待会耗尽 tokio 工作线程
/// - 关注点分离：prepare = 触发，status = 查询，playlist/asset = 纯服务
pub async fn hls_playlist_handler(
    State(state): State<AppState>,
    AuthenticatedUserQuery(user_id): AuthenticatedUserQuery,
    Path(file_id): Path<Uuid>,
) -> Result<Response, AppError> {
    // 为什么返回 m3u8 占位而非 503 JSON：
    // hls.js 将 m3u8/ts 拉取视为媒体流水线的一部分，收到 503 JSON 会触发 fatal 错误；
    // 返回最小 m3u8 + Retry-After 可实现温和重试与更好的播放体验。
    let _file = state.file_service.get_file(file_id, user_id).await?;
    let out_dir = state.file_service.hls_output_dir(file_id);
    let playlist_path = out_dir.join("playlist.m3u8");
    let master_path = out_dir.join("master.m3u8");

    let exists_master = tokio::fs::try_exists(&master_path)
        .await
        .map_err(|e| AppError::File(e.to_string()))?;
    let exists_playlist = tokio::fs::try_exists(&playlist_path)
        .await
        .map_err(|e| AppError::File(e.to_string()))?;

    if !exists_master && !exists_playlist {
        // HLS 尚未就绪，返回占位响应；前端 hls.js 会按 Retry-After 重试
        return Ok(hls_processing_response(2));
    }

    let pick = if exists_master {
        master_path
    } else {
        playlist_path
    };
    let data = tokio::fs::read(&pick)
        .await
        .map_err(|e| AppError::File(format!("读取 HLS 列表失败: {}", e)))?;
    let rewritten = rewrite_hls_segment_refs(&data);
    crate::utils::response::file_response(
        rewritten,
        "playlist.m3u8",
        "application/vnd.apple.mpegurl",
        true,
    )
    .map_err(|_| AppError::Internal)
}

/// 将 m3u8 内容中的 segment*.ts 行改为 hls/segment*.ts，供相对 URL 正确解析。
fn rewrite_hls_segment_refs(data: &[u8]) -> Vec<u8> {
    use std::io::{BufRead, BufReader, Write};
    let mut out = Vec::with_capacity(data.len() + 64);
    for line in BufReader::new(data).lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && (trimmed.ends_with(".ts") || trimmed.ends_with(".m3u8"))
            && trimmed
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '/')
            && !trimmed.contains("..")
            && !trimmed.starts_with("hls/")
        {
            let _ = writeln!(out, "hls/{}", trimmed);
        } else {
            let _ = writeln!(out, "{}", line);
        }
    }
    out
}

/// 返回 HLS 分片（segment*.ts）或子 playlist.m3u8。仅允许安全文件名，防止路径穿越。
///
/// 职责边界：纯服务端，不触发转码。转码由 worker 异步完成后产物落盘，这里只读盘返回。
pub async fn hls_asset_handler(
    State(state): State<AppState>,
    AuthenticatedUserQuery(user_id): AuthenticatedUserQuery,
    Path((file_id, path)): Path<(Uuid, String)>,
) -> Result<Response, AppError> {
    let file = state.file_service.get_file(file_id, user_id).await?;
    if !state.file_service.should_use_hls(&file).await {
        return Err(AppError::NotFound);
    }
    let out_dir = state.file_service.hls_output_dir(file_id);

    // 路径安全校验（防止目录穿越）
    use std::path::Component;
    if path.is_empty() || path.contains("..") {
        return Err(AppError::Validation("无效的 HLS 资源名".to_string()));
    }
    let ok_ext = path.ends_with(".ts") || path.ends_with(".m3u8");
    if !ok_ext {
        return Err(AppError::Validation("无效的 HLS 资源名".to_string()));
    }
    let safe_components = std::path::Path::new(&path).components().all(|c| match c {
        Component::Normal(s) => s
            .to_string_lossy()
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '_'),
        _ => false,
    });
    if !safe_components {
        return Err(AppError::Validation("无效的 HLS 资源名".to_string()));
    }

    let fs_path = out_dir.join(&path);
    let data = tokio::fs::read(&fs_path)
        .await
        .map_err(|_| AppError::NotFound)?;
    let mime = if path.ends_with(".m3u8") {
        "application/vnd.apple.mpegurl"
    } else {
        "video/MP2T"
    };
    let filename = path.rsplit('/').next().unwrap_or(&path);
    crate::utils::response::file_response(data, filename, mime, true)
        .map_err(|_| AppError::Internal)
}
