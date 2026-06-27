//! # 文件夹数据访问层
//!
//! 提供文件夹表的所有数据库操作。

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::folder::Folder;
use crate::utils::AppError;

/// 文件夹仓库
pub struct FoldersRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> FoldersRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // 查询方法
    // ========================================================================

    /// 根据 ID 获取文件夹
    pub async fn find_by_id(
        &self,
        folder_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Folder>, AppError> {
        sqlx::query_as::<_, Folder>(
            "SELECT id, user_id, name, parent_id, created_at, updated_at FROM folders WHERE id = $1 AND user_id = $2",
        )
        .bind(folder_id)
        .bind(user_id)
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::from)
    }

    /// 验证文件夹是否存在（仅返回 ID）
    pub async fn exists(&self, folder_id: Uuid, user_id: Uuid) -> Result<bool, AppError> {
        let result: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM folders WHERE id = $1 AND user_id = $2")
                .bind(folder_id)
                .bind(user_id)
                .fetch_optional(self.pool)
                .await?;
        Ok(result.is_some())
    }

    /// 列出指定父文件夹下的所有文件夹
    pub async fn list_by_parent(
        &self,
        user_id: Uuid,
        parent_id: Option<Uuid>,
    ) -> Result<Vec<Folder>, AppError> {
        if parent_id.is_some() {
            sqlx::query_as::<_, Folder>(
                "SELECT id, user_id, name, parent_id, created_at, updated_at FROM folders WHERE user_id = $1 AND parent_id = $2 ORDER BY name",
            )
            .bind(user_id)
            .bind(parent_id)
            .fetch_all(self.pool)
            .await
            .map_err(AppError::from)
        } else {
            sqlx::query_as::<_, Folder>(
                "SELECT id, user_id, name, parent_id, created_at, updated_at FROM folders WHERE user_id = $1 AND parent_id IS NULL ORDER BY name",
            )
            .bind(user_id)
            .fetch_all(self.pool)
            .await
            .map_err(AppError::from)
        }
    }

    /// 查找指定父文件夹下的同名文件夹。
    pub async fn find_by_name_and_parent(
        &self,
        user_id: Uuid,
        parent_id: Option<Uuid>,
        name: &str,
    ) -> Result<Option<Folder>, AppError> {
        sqlx::query_as::<_, Folder>(
            r#"
            SELECT id, user_id, name, parent_id, created_at, updated_at
            FROM folders
            WHERE user_id = $1
              AND parent_id IS NOT DISTINCT FROM $2
              AND name = $3
            "#,
        )
        .bind(user_id)
        .bind(parent_id)
        .bind(name)
        .fetch_optional(self.pool)
        .await
        .map_err(AppError::from)
    }

    /// 检查同级目录下是否存在同名文件夹
    pub async fn name_exists_in_parent(
        &self,
        user_id: Uuid,
        parent_id: Option<Uuid>,
        name: &str,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, AppError> {
        let result: Option<Uuid> = if let Some(exclude) = exclude_id {
            sqlx::query_scalar(
                "SELECT id FROM folders WHERE user_id = $1 AND parent_id IS NOT DISTINCT FROM $2 AND name = $3 AND id != $4",
            )
            .bind(user_id)
            .bind(parent_id)
            .bind(name)
            .bind(exclude)
            .fetch_optional(self.pool)
            .await?
        } else {
            sqlx::query_scalar(
                "SELECT id FROM folders WHERE user_id = $1 AND parent_id IS NOT DISTINCT FROM $2 AND name = $3",
            )
            .bind(user_id)
            .bind(parent_id)
            .bind(name)
            .fetch_optional(self.pool)
            .await?
        };
        Ok(result.is_some())
    }

    /// 获取文件夹路径（从根到当前文件夹）
    pub async fn get_path(&self, folder_id: Uuid, user_id: Uuid) -> Result<Vec<Folder>, AppError> {
        sqlx::query_as::<_, Folder>(
            r#"
            WITH RECURSIVE folder_path AS (
                SELECT id, user_id, name, parent_id, created_at, updated_at, 0 as depth
                FROM folders WHERE id = $1 AND user_id = $2
                UNION ALL
                SELECT f.id, f.user_id, f.name, f.parent_id, f.created_at, f.updated_at, fp.depth + 1
                FROM folders f
                INNER JOIN folder_path fp ON f.id = fp.parent_id
            )
            SELECT id, user_id, name, parent_id, created_at, updated_at
            FROM folder_path
            ORDER BY depth DESC
            "#,
        )
        .bind(folder_id)
        .bind(user_id)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::from)
    }

    /// 获取所有子文件夹 ID（递归）
    pub async fn get_all_descendant_ids(
        &self,
        folder_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            WITH RECURSIVE all_folders AS (
                SELECT id FROM folders WHERE id = $1 AND user_id = $2
                UNION ALL
                SELECT f.id FROM folders f
                INNER JOIN all_folders af ON f.parent_id = af.id
            )
            SELECT id FROM all_folders
            "#,
        )
        .bind(folder_id)
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;
        Ok(ids.into_iter().map(|(id,)| id).collect())
    }

    /// 检查目标文件夹是否是源文件夹的后代（防止循环引用）
    ///
    /// 限定在同一用户的文件夹树内，避免跨用户误判。
    pub async fn is_descendant_of(
        &self,
        source_id: Uuid,
        target_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, AppError> {
        let result: Option<i64> = sqlx::query_scalar(
            r#"
            WITH RECURSIVE descendants AS (
                SELECT id FROM folders WHERE parent_id = $1 AND user_id = $3
                UNION ALL
                SELECT f.id FROM folders f
                INNER JOIN descendants d ON f.parent_id = d.id
                WHERE f.user_id = $3
            )
            SELECT 1 FROM descendants WHERE id = $2
            "#,
        )
        .bind(source_id)
        .bind(target_id)
        .bind(user_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(result.is_some())
    }

    // ========================================================================
    // 写入方法
    // ========================================================================

    /// 创建文件夹
    pub async fn create(
        &self,
        user_id: Uuid,
        name: &str,
        parent_id: Option<Uuid>,
    ) -> Result<Folder, AppError> {
        sqlx::query_as::<_, Folder>(
            r#"
            INSERT INTO folders (user_id, name, parent_id)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, name, parent_id, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(name)
        .bind(parent_id)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::from)
    }

    /// 重命名文件夹
    pub async fn rename(
        &self,
        folder_id: Uuid,
        user_id: Uuid,
        name: &str,
    ) -> Result<Folder, AppError> {
        sqlx::query_as::<_, Folder>(
            r#"
            UPDATE folders
            SET name = $1, updated_at = CURRENT_TIMESTAMP
            WHERE id = $2 AND user_id = $3
            RETURNING id, user_id, name, parent_id, created_at, updated_at
            "#,
        )
        .bind(name)
        .bind(folder_id)
        .bind(user_id)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::from)
    }

    /// 移动文件夹
    pub async fn move_to(
        &self,
        folder_id: Uuid,
        user_id: Uuid,
        new_parent_id: Option<Uuid>,
    ) -> Result<Folder, AppError> {
        sqlx::query_as::<_, Folder>(
            r#"
            UPDATE folders
            SET parent_id = $1, updated_at = CURRENT_TIMESTAMP
            WHERE id = $2 AND user_id = $3
            RETURNING id, user_id, name, parent_id, created_at, updated_at
            "#,
        )
        .bind(new_parent_id)
        .bind(folder_id)
        .bind(user_id)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::from)
    }

    /// 删除文件夹（级联删除由数据库处理）
    ///
    /// **必须**传入已按 `user_id` 过滤过的 `folder_ids`，此处再次加 `user_id` 条件防止跨用户误删。
    pub async fn delete(&self, folder_ids: &[Uuid], user_id: Uuid) -> Result<u64, AppError> {
        let result = sqlx::query("DELETE FROM folders WHERE id = ANY($1) AND user_id = $2")
            .bind(folder_ids)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// 获取文件夹下的文件数量。
    ///
    /// 当前文件夹相关 API 还未对外暴露「带计数的树状视图」或
    /// 「按文件夹统计使用量」接口，因此暂未用到该方法。
    /// 未来如果在前端展示「每个文件夹内的文件数」或做配额报表，
    /// 可以直接调用这里的聚合查询。
    ///
    /// **约定**：`folder_ids` 须为已按 `user_id` 过滤过的 ID 列表；此处再加 `user_id` 条件防御。
    #[allow(dead_code)]
    pub async fn count_files_in_folders(
        &self,
        folder_ids: &[Uuid],
        user_id: Uuid,
    ) -> Result<i64, AppError> {
        let result: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM files WHERE folder_id = ANY($1) AND user_id = $2 AND deleted_at IS NULL")
                .bind(folder_ids)
                .bind(user_id)
                .fetch_one(self.pool)
                .await?;
        Ok(result.0)
    }

    /// 获取文件夹下所有文件的路径
    ///
    /// **约定**：`folder_ids` 须为已按 `user_id` 过滤过的 ID 列表；此处再加 `user_id` 条件防止跨用户泄露。
    pub async fn get_file_paths_in_folders(
        &self,
        folder_ids: &[Uuid],
        user_id: Uuid,
    ) -> Result<Vec<String>, AppError> {
        let paths: Vec<(String,)> = sqlx::query_as(
            "SELECT file_path FROM files WHERE folder_id = ANY($1) AND user_id = $2 AND deleted_at IS NULL",
        )
        .bind(folder_ids)
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;
        Ok(paths.into_iter().map(|(p,)| p).collect())
    }

    /// 删除文件夹下的所有文件记录
    ///
    /// **约定**：`folder_ids` 须为已按 `user_id` 过滤过的 ID 列表；此处再加 `user_id` 条件防止跨用户误删。
    pub async fn delete_files_in_folders(
        &self,
        folder_ids: &[Uuid],
        user_id: Uuid,
    ) -> Result<u64, AppError> {
        let result = sqlx::query("DELETE FROM files WHERE folder_id = ANY($1) AND user_id = $2")
            .bind(folder_ids)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn find_move_file_name_conflict(
        &self,
        user_id: Uuid,
        file_ids: &[Uuid],
        folder_id: Option<Uuid>,
    ) -> Result<Option<String>, AppError> {
        let conflict: Option<String> = sqlx::query_scalar(
            r#"
            WITH moving AS (
                SELECT id, original_filename
                FROM files
                WHERE id = ANY($1) AND user_id = $2 AND deleted_at IS NULL
            ),
            dupes AS (
                SELECT original_filename
                FROM moving
                GROUP BY original_filename
                HAVING COUNT(*) > 1
            ),
            conflicts AS (
                SELECT f.original_filename
                FROM files f
                JOIN moving m ON f.original_filename = m.original_filename
                WHERE f.user_id = $2
                  AND f.folder_id IS NOT DISTINCT FROM $3
                  AND f.id <> ALL($1)
                  AND f.deleted_at IS NULL
            )
            SELECT original_filename FROM dupes
            UNION
            SELECT original_filename FROM conflicts
            LIMIT 1
            "#,
        )
        .bind(file_ids)
        .bind(user_id)
        .bind(folder_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(conflict)
    }

    /// 移动文件到文件夹
    pub async fn move_files_to_folder(
        &self,
        user_id: Uuid,
        file_ids: &[Uuid],
        folder_id: Option<Uuid>,
    ) -> Result<u64, AppError> {
        let result = sqlx::query(
            "UPDATE files SET folder_id = $1, updated_at = CURRENT_TIMESTAMP WHERE id = ANY($2) AND user_id = $3 AND deleted_at IS NULL",
        )
        .bind(folder_id)
        .bind(file_ids)
        .bind(user_id)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// 获取多个文件夹下所有文件的 ID（递归）
    pub async fn get_all_file_ids_in_folders(
        &self,
        user_id: Uuid,
        folder_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, AppError> {
        if folder_ids.is_empty() {
            return Ok(vec![]);
        }

        // 递归获取所有子文件夹 ID
        let all_folder_ids: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            WITH RECURSIVE all_folders AS (
                SELECT id FROM folders WHERE id = ANY($1) AND user_id = $2
                UNION ALL
                SELECT f.id FROM folders f
                INNER JOIN all_folders af ON f.parent_id = af.id
            )
            SELECT id FROM all_folders
            "#,
        )
        .bind(folder_ids)
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;

        let folder_id_list: Vec<Uuid> = all_folder_ids.into_iter().map(|(id,)| id).collect();

        // 获取所有文件 ID
        let file_ids: Vec<(Uuid,)> =
            sqlx::query_as("SELECT id FROM files WHERE folder_id = ANY($1) AND user_id = $2 AND deleted_at IS NULL")
                .bind(&folder_id_list)
                .bind(user_id)
                .fetch_all(self.pool)
                .await?;

        Ok(file_ids.into_iter().map(|(id,)| id).collect())
    }
}
