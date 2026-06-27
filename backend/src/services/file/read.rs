//! 文件读取（元数据 / 内容 / 流式读取）

use std::path::Path;
use uuid::Uuid;

use crate::models::file::File;
use crate::services::storage::StorageReadStream;
use crate::utils::AppError;

use super::FileService;

impl FileService {
    pub async fn get_file(&self, file_id: Uuid, user_id: Uuid) -> Result<File, AppError> {
        self.files_repo
            .find_by_id(file_id, user_id)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn find_file_by_name_and_folder(
        &self,
        user_id: Uuid,
        filename: &str,
        folder_id: Option<Uuid>,
    ) -> Result<Option<File>, AppError> {
        self.files_repo
            .find_by_name_and_folder(user_id, filename, folder_id)
            .await
    }

    pub async fn get_file_for_thumbnail(
        &self,
        file_id: Uuid,
        user_id: Uuid,
    ) -> Result<File, AppError> {
        if let Some(file) = self.files_repo.find_by_id(file_id, user_id).await? {
            return Ok(file);
        }

        self.files_repo
            .find_deleted_by_id(file_id, user_id)
            .await?
            .ok_or(AppError::NotFound)
    }

    /// 检查文件在存储中是否存在且非空
    /// 用于验证缓存有效性，防止数据库记录存在但文件缺失的情况
    pub async fn verify_file_exists(&self, file: &File) -> Result<(), AppError> {
        let path = Path::new(&file.file_path);

        // 检查文件是否存在
        match tokio::fs::metadata(path).await {
            Ok(metadata) => {
                // 检查是否是文件且非空
                if !metadata.is_file() {
                    return Err(AppError::NotFound);
                }
                if metadata.len() == 0 {
                    return Err(AppError::NotFound);
                }
                Ok(())
            }
            Err(_) => Err(AppError::NotFound),
        }
    }

    pub async fn get_file_data(&self, file: &File) -> Result<Vec<u8>, AppError> {
        self.storage.get_file(&file.file_path).await
    }

    pub async fn open_file_stream(&self, file: &File) -> Result<StorageReadStream, AppError> {
        self.storage.open_read_stream(&file.file_path).await
    }

    pub async fn open_file_stream_range(
        &self,
        file: &File,
        start: u64,
        end_inclusive: u64,
    ) -> Result<StorageReadStream, AppError> {
        self.storage
            .open_read_stream_range(&file.file_path, start, end_inclusive)
            .await
    }

    /// 读取已生成的缩略图（方案 B：先读盘），按用户隔离存放。
    pub async fn get_thumbnail(&self, file_id: Uuid, user_id: Uuid) -> Result<Vec<u8>, AppError> {
        self.storage.get_thumbnail(file_id, user_id).await
    }

    /// 保存缩略图（方案 B：首次生成后写盘），按用户隔离存放。
    pub async fn save_thumbnail(
        &self,
        file_id: Uuid,
        user_id: Uuid,
        data: &[u8],
    ) -> Result<(), AppError> {
        self.storage.save_thumbnail(file_id, user_id, data).await
    }

    /// 删除缩略图（如原文件删除时）
    pub async fn delete_thumbnail(&self, file_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        self.storage.delete_thumbnail(file_id, user_id).await
    }
}
