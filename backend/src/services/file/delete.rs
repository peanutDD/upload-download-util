//! 删除相关（软删除 / 回收站 / 彻底删除）

use std::collections::{HashMap, HashSet};

use futures::StreamExt;

use crate::models::file::{BatchTrashFailure, BatchTrashResult, FileResponse};
use uuid::Uuid;

use crate::utils::AppError;

use super::FileService;

const MIN_TRASH_RETENTION_DAYS: i64 = 1;
const MAX_TRASH_RETENTION_DAYS: i64 = 3650;
const MIN_TRASH_PURGE_BATCH_LIMIT: i64 = 1;
const MAX_TRASH_PURGE_BATCH_LIMIT: i64 = 10_000;

fn validate_purge_expired_trash_input(
    retention_days: i64,
    batch_limit: i64,
) -> Result<(), AppError> {
    if !(MIN_TRASH_RETENTION_DAYS..=MAX_TRASH_RETENTION_DAYS).contains(&retention_days) {
        return Err(AppError::Validation(format!(
            "retention_days must be between {} and {}",
            MIN_TRASH_RETENTION_DAYS, MAX_TRASH_RETENTION_DAYS
        )));
    }

    if !(MIN_TRASH_PURGE_BATCH_LIMIT..=MAX_TRASH_PURGE_BATCH_LIMIT).contains(&batch_limit) {
        return Err(AppError::Validation(format!(
            "batch_limit must be between {} and {}",
            MIN_TRASH_PURGE_BATCH_LIMIT, MAX_TRASH_PURGE_BATCH_LIMIT
        )));
    }

    Ok(())
}

impl FileService {
    pub async fn delete_file(&self, file_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        self.get_file(file_id, user_id).await?;
        let affected = self.files_repo.soft_delete(file_id, user_id).await?;
        if affected == 0 {
            return Err(AppError::NotFound);
        }
        self.enqueue_fulltext_remove_task_best_effort(file_id).await;
        Ok(())
    }

    pub async fn batch_delete(&self, ids: &[Uuid], user_id: Uuid) -> Result<u64, AppError> {
        let affected = self.files_repo.soft_delete_batch(ids, user_id).await?;
        for file_id in ids {
            self.enqueue_fulltext_remove_task_best_effort(*file_id)
                .await;
        }
        Ok(affected)
    }

    pub async fn list_trash(&self, user_id: Uuid) -> Result<Vec<FileResponse>, AppError> {
        let files = self.files_repo.list_deleted(user_id).await?;
        Ok(files.into_iter().map(FileResponse::from).collect())
    }

    pub async fn restore_file(
        &self,
        file_id: Uuid,
        user_id: Uuid,
    ) -> Result<FileResponse, AppError> {
        let file = self
            .files_repo
            .find_deleted_by_id(file_id, user_id)
            .await?
            .ok_or(AppError::NotFound)?;

        if let Some(conflict) = self
            .files_repo
            .find_by_name_and_folder(user_id, &file.original_filename, file.folder_id)
            .await?
        {
            if conflict.id != file_id {
                return Err(AppError::Validation("同名文件已存在".to_string()));
            }
        }

        let restored = self.files_repo.restore_deleted(file_id, user_id).await?;
        self.enqueue_fulltext_index_task_best_effort(file_id, user_id)
            .await;
        Ok(FileResponse::from(restored))
    }

    pub async fn batch_restore_files(
        &self,
        ids: &[Uuid],
        user_id: Uuid,
    ) -> Result<BatchTrashResult, AppError> {
        let mut succeeded = 0;
        let mut failed = Vec::new();

        for id in ids {
            match self.restore_file(*id, user_id).await {
                Ok(_) => succeeded += 1,
                Err(error) => failed.push(BatchTrashFailure {
                    id: *id,
                    message: error.to_string(),
                }),
            }
        }

        Ok(BatchTrashResult { succeeded, failed })
    }

    pub async fn permanently_delete_file(
        &self,
        file_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let file = self
            .files_repo
            .find_deleted_by_id(file_id, user_id)
            .await?
            .ok_or(AppError::NotFound)?;
        self.cleanup_unreferenced_deleted_files(std::slice::from_ref(&file), false)
            .await?;
        let affected = self
            .files_repo
            .hard_delete_deleted(file_id, user_id)
            .await?;
        if affected == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }

    pub async fn batch_permanently_delete_files(
        &self,
        ids: &[Uuid],
        user_id: Uuid,
    ) -> Result<BatchTrashResult, AppError> {
        let mut files = Vec::new();
        let mut failed = Vec::new();
        let mut seen_ids = HashSet::new();

        for id in ids {
            if !seen_ids.insert(*id) {
                continue;
            }

            match self.files_repo.find_deleted_by_id(*id, user_id).await? {
                Some(file) => files.push(file),
                None => failed.push(BatchTrashFailure {
                    id: *id,
                    message: AppError::NotFound.to_string(),
                }),
            }
        }

        self.cleanup_unreferenced_deleted_files(&files, false)
            .await?;

        let file_ids = files.iter().map(|file| file.id).collect::<Vec<_>>();
        let deleted_files = self
            .files_repo
            .hard_delete_deleted_batch(&file_ids, user_id)
            .await?;
        let deleted_ids: HashSet<Uuid> = deleted_files.iter().map(|file| file.id).collect();
        failed.extend(
            file_ids
                .iter()
                .filter(|id| !deleted_ids.contains(id))
                .map(|id| BatchTrashFailure {
                    id: *id,
                    message: AppError::NotFound.to_string(),
                }),
        );
        let succeeded = deleted_files.len() as u64;

        Ok(BatchTrashResult { succeeded, failed })
    }

    pub async fn empty_trash(&self, user_id: Uuid) -> Result<u64, AppError> {
        let mut deleted = 0;

        loop {
            let files = self
                .files_repo
                .list_deleted(user_id)
                .await?
                .into_iter()
                .take(MAX_TRASH_PURGE_BATCH_LIMIT as usize)
                .collect::<Vec<_>>();

            if files.is_empty() {
                break;
            }

            self.cleanup_unreferenced_deleted_files(&files, false)
                .await?;

            let ids = files.iter().map(|file| file.id).collect::<Vec<_>>();
            deleted += self
                .files_repo
                .hard_delete_deleted_batch(&ids, user_id)
                .await?
                .len() as u64;

            if files.len() < MAX_TRASH_PURGE_BATCH_LIMIT as usize {
                break;
            }
        }

        Ok(deleted)
    }

    pub async fn purge_expired_trash(
        &self,
        retention_days: i64,
        batch_limit: i64,
    ) -> Result<u64, AppError> {
        validate_purge_expired_trash_input(retention_days, batch_limit)?;

        let files = self
            .files_repo
            .purge_expired_deleted(retention_days, batch_limit)
            .await?;
        let deleted = files.len() as u64;
        self.cleanup_unreferenced_deleted_files(&files, true)
            .await?;
        Ok(deleted)
    }

    async fn cleanup_unreferenced_deleted_files(
        &self,
        files: &[crate::entities::file::File],
        rows_already_deleted: bool,
    ) -> Result<(), AppError> {
        let mut purge_counts: HashMap<_, HashSet<_>> = HashMap::new();
        for file in files {
            purge_counts
                .entry(file.file_path.clone())
                .or_default()
                .insert(file.id);
        }

        let paths = purge_counts.keys().cloned().collect::<Vec<_>>();
        let ref_counts = self.files_repo.count_by_file_paths(&paths).await?;

        let derived_targets = files
            .iter()
            .map(|file| (file.id, file.user_id))
            .collect::<Vec<_>>();
        let storage_paths = paths
            .into_iter()
            .filter(|path| {
                let total_refs = ref_counts.get(path).copied().unwrap_or(0);
                let purging_refs = purge_counts
                    .get(path)
                    .map(|file_ids| file_ids.len() as u64)
                    .unwrap_or(0);
                let remaining_refs = if rows_already_deleted {
                    total_refs
                } else {
                    total_refs.saturating_sub(purging_refs)
                };
                remaining_refs == 0
            })
            .collect::<Vec<_>>();

        let derived_results = futures::stream::iter(derived_targets)
            .map(|(file_id, user_id)| self.delete_derived_assets(file_id, user_id))
            .buffer_unordered(10)
            .collect::<Vec<_>>()
            .await;

        let storage_results = futures::stream::iter(storage_paths)
            .map(|path| async move { self.storage.delete_file(&path).await })
            .buffer_unordered(10)
            .collect::<Vec<_>>()
            .await;

        let errors = derived_results
            .into_iter()
            .chain(storage_results)
            .filter_map(Result::err)
            .map(|error| error.to_string())
            .collect::<Vec<_>>();

        if !errors.is_empty() {
            return Err(AppError::Storage(format!(
                "deleted file cleanup failed: {}",
                errors.join("; ")
            )));
        }

        Ok(())
    }

    async fn delete_derived_assets(&self, file_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        self.delete_thumbnail(file_id, user_id).await?;
        self.delete_hls(file_id).await?;
        self.delete_gif_preview_video(file_id).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::validate_purge_expired_trash_input;

    #[test]
    fn purge_expired_trash_rejects_unsafe_retention_and_batch_limits() {
        assert!(validate_purge_expired_trash_input(30, 500).is_ok());
        assert!(validate_purge_expired_trash_input(0, 500).is_err());
        assert!(validate_purge_expired_trash_input(-1, 500).is_err());
        assert!(validate_purge_expired_trash_input(30, 0).is_err());
        assert!(validate_purge_expired_trash_input(30, -1).is_err());
        assert!(validate_purge_expired_trash_input(3651, 500).is_err());
        assert!(validate_purge_expired_trash_input(30, 10_001).is_err());
    }
}
