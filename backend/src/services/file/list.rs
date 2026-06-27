//! 文件列表查询（分页/过滤/搜索）

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

use deadpool_redis::{redis::cmd, Pool};
use serde_json::json;
use uuid::Uuid;

use crate::config::CacheConfig;
use crate::models::file::{File, FileListQuery, FileResponse};
use crate::services::redis::RedisService;
use crate::utils::crypto::sha256_hex;
use crate::utils::{is_macos_appledouble_filename, AppError};

use super::FileService;

const LOCAL_STORAGE_EXISTENCE_TTL: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
struct CachedStorageExistence {
    exists: bool,
    checked_at: Instant,
}

static LOCAL_STORAGE_EXISTENCE_CACHE: OnceLock<RwLock<HashMap<String, CachedStorageExistence>>> =
    OnceLock::new();

#[derive(serde::Serialize, serde::Deserialize)]
struct CachedFileListResponse {
    files: Vec<FileResponse>,
    total: Option<u64>,
    next_cursor: Option<String>,
}

impl FileService {
    pub async fn list_by_folder(
        &self,
        user_id: Uuid,
        folder_id: Option<Uuid>,
    ) -> Result<Vec<crate::models::file::File>, AppError> {
        let files = self.files_repo.list_by_folder(user_id, folder_id).await?;
        Ok(self.visible_existing_files(files).await.0)
    }

    pub async fn list_files(
        &self,
        user_id: Uuid,
        query: FileListQuery,
    ) -> Result<(Vec<FileResponse>, Option<u64>, Option<String>), AppError> {
        let result = self.files_repo.list(user_id, query).await?;
        let (files, hidden_count) = self.visible_existing_files(result.files).await;
        let total = result
            .total
            .map(|t| (t as u64).saturating_sub(hidden_count as u64));
        Ok((
            files.into_iter().map(FileResponse::from).collect(),
            total,
            result.next_cursor,
        ))
    }

    async fn visible_existing_files(&self, files: Vec<File>) -> (Vec<File>, usize) {
        let mut visible = Vec::with_capacity(files.len());
        let mut hidden_count = 0usize;

        for file in files {
            if is_macos_appledouble_filename(&file.original_filename) {
                hidden_count += 1;
                tracing::info!(
                    file_id = %file.id,
                    user_id = %file.user_id,
                    filename = %file.original_filename,
                    "hid macOS AppleDouble file record from listing"
                );
                continue;
            }

            if file.storage_backend.eq_ignore_ascii_case("local") {
                if let Some(exists) = Self::cached_local_storage_exists(&file.file_path) {
                    if exists {
                        visible.push(file);
                    } else {
                        hidden_count += 1;
                        tracing::warn!(
                            file_id = %file.id,
                            user_id = %file.user_id,
                            file_path = %file.file_path,
                            "hid local file record because cached storage object is missing"
                        );
                    }
                    continue;
                }

                match self.storage.open_read_stream(&file.file_path).await {
                    Ok(_) => {
                        Self::cache_local_storage_exists(&file.file_path, true);
                        visible.push(file);
                    }
                    Err(AppError::NotFound) => {
                        Self::cache_local_storage_exists(&file.file_path, false);
                        hidden_count += 1;
                        tracing::warn!(
                            file_id = %file.id,
                            user_id = %file.user_id,
                            file_path = %file.file_path,
                            "hid local file record because storage object is missing"
                        );
                    }
                    Err(err) => {
                        tracing::warn!(
                            file_id = %file.id,
                            user_id = %file.user_id,
                            file_path = %file.file_path,
                            error = %err,
                            "kept file record despite storage existence check error"
                        );
                        visible.push(file);
                    }
                }
            } else {
                visible.push(file);
            }
        }

        (visible, hidden_count)
    }

    fn cached_local_storage_exists(file_path: &str) -> Option<bool> {
        let cache = LOCAL_STORAGE_EXISTENCE_CACHE.get_or_init(|| RwLock::new(HashMap::new()));
        let cached = cache.read().ok()?.get(file_path).copied()?;

        if cached.checked_at.elapsed() <= LOCAL_STORAGE_EXISTENCE_TTL {
            Some(cached.exists)
        } else {
            None
        }
    }

    fn cache_local_storage_exists(file_path: &str, exists: bool) {
        let cache = LOCAL_STORAGE_EXISTENCE_CACHE.get_or_init(|| RwLock::new(HashMap::new()));

        if let Ok(mut cache) = cache.write() {
            cache.insert(
                file_path.to_string(),
                CachedStorageExistence {
                    exists,
                    checked_at: Instant::now(),
                },
            );
        }
    }

    pub async fn list_files_cached(
        &self,
        user_id: Uuid,
        query: FileListQuery,
        redis_pool: Option<&Pool>,
        cache_config: &CacheConfig,
    ) -> Result<(Vec<FileResponse>, Option<u64>, Option<String>), AppError> {
        let page = query.page.unwrap_or(1);
        let is_cursor_pagination =
            query.cursor.is_some() || matches!(query.pagination.as_deref(), Some("cursor"));

        if cache_config.enabled
            && page == 1
            && !is_cursor_pagination
            && query.include_total.unwrap_or(true)
        {
            if let Some(pool) = redis_pool {
                let fingerprint = format!(
                    "visibility=v2&page={:?}&limit={:?}&pagination={:?}&cursor={:?}&search={:?}&mime_type={:?}&category={:?}&folder_id={:?}&date_from={:?}&date_to={:?}&size_min={:?}&size_max={:?}&sort_by={:?}&sort_order={:?}&include_total={:?}",
                    query.page,
                    query.limit,
                    query.pagination,
                    query.cursor,
                    query.search,
                    query.mime_type,
                    query.category,
                    query.folder_id,
                    query.date_from,
                    query.date_to,
                    query.size_min,
                    query.size_max,
                    query.sort_by,
                    query.sort_order,
                    query.include_total,
                );
                let hash = sha256_hex(fingerprint.as_bytes());
                let redis = RedisService::new(pool.clone());
                let ver = redis.get_user_cache_version(user_id).await.unwrap_or(1);
                let cache_key = format!("cache:files:list:{}:{}:{}", user_id, ver, hash);

                if let Ok(mut conn) = pool.get().await {
                    let cached: Result<Option<String>, _> =
                        cmd("GET").arg(&cache_key).query_async(&mut conn).await;
                    if let Ok(Some(s)) = cached {
                        if let Ok(cached_response) =
                            serde_json::from_str::<CachedFileListResponse>(&s)
                        {
                            return Ok((
                                cached_response.files,
                                cached_response.total,
                                cached_response.next_cursor,
                            ));
                        }
                    }
                }

                let (files, total, next_cursor) = self.list_files(user_id, query.clone()).await?;
                let response_json = json!({
                    "files": files,
                    "total": total,
                    "next_cursor": next_cursor,
                });

                if let Ok(mut conn) = pool.get().await {
                    if let Ok(body) = serde_json::to_string(&response_json) {
                        let _: Result<(), _> = cmd("SETEX")
                            .arg(&cache_key)
                            .arg(cache_config.list_ttl_secs)
                            .arg(body)
                            .query_async(&mut conn)
                            .await;
                    }
                }
                return Ok((files, total, next_cursor));
            }
        }

        self.list_files(user_id, query).await
    }
}

#[cfg(test)]
mod tests {
    use super::CachedFileListResponse;

    #[test]
    fn cached_file_list_response_deserializes_from_response_shape() {
        let file = crate::models::file::FileResponse {
            id: uuid::Uuid::new_v4(),
            filename: "a.txt".to_string(),
            original_filename: "a.txt".to_string(),
            file_size: 1,
            mime_type: "text/plain".to_string(),
            category: None,
            folder_id: None,
            created_at: chrono::Utc::now(),
            deleted_at: None,
        };

        let body = serde_json::json!({
            "files": [file],
            "total": 1u64,
            "next_cursor": null,
        })
        .to_string();

        let parsed = serde_json::from_str::<CachedFileListResponse>(&body).unwrap();
        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.total, Some(1));
        assert!(parsed.next_cursor.is_none());
    }
}
