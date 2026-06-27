//! # API Token 数据访问层
//!
//! 提供 API Token 表的所有数据库操作。

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::api_token::{ApiToken, ApiTokenListItem};
use crate::utils::AppError;

// 确保 ApiToken 被导入用于查询

pub struct CreateApiTokenRecord<'a> {
    pub user_id: Uuid,
    pub name: &'a str,
    pub token_hash: &'a str,
    pub token_prefix: &'a str,
    pub expires_at: Option<DateTime<Utc>>,
    pub webdav_enabled: bool,
    pub webdav_read_only: bool,
    pub webdav_root_folder_id: Option<Uuid>,
}

/// API Token 仓库
pub struct ApiTokensRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ApiTokensRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // 查询方法
    // ========================================================================

    /// 根据 token hash 获取 API Token
    pub async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<ApiToken>, AppError> {
        sqlx::query_as::<_, ApiToken>("SELECT * FROM api_tokens WHERE token_hash = $1")
            .bind(token_hash)
            .fetch_optional(self.pool)
            .await
            .map_err(AppError::from)
    }

    /// 列出用户的所有 API Token（不含 hash）
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<ApiTokenListItem>, AppError> {
        let tokens = sqlx::query_as::<_, ApiToken>(
            "SELECT * FROM api_tokens WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await
        .map_err(AppError::from)?;

        Ok(tokens.into_iter().map(ApiTokenListItem::from).collect())
    }

    // ========================================================================
    // 写入方法
    // ========================================================================

    /// 创建 API Token
    pub async fn create(&self, record: CreateApiTokenRecord<'_>) -> Result<ApiToken, AppError> {
        sqlx::query_as::<_, ApiToken>(
            r#"
            INSERT INTO api_tokens (
                user_id, name, token_hash, token_prefix, expires_at,
                webdav_enabled, webdav_read_only, webdav_root_folder_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(record.user_id)
        .bind(record.name)
        .bind(record.token_hash)
        .bind(record.token_prefix)
        .bind(record.expires_at)
        .bind(record.webdav_enabled)
        .bind(record.webdav_read_only)
        .bind(record.webdav_root_folder_id)
        .fetch_one(self.pool)
        .await
        .map_err(AppError::from)
    }

    /// 更新最后使用时间
    pub async fn update_last_used(&self, token_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE api_tokens SET last_used_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(token_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// 删除 API Token
    pub async fn delete(&self, token_id: Uuid, user_id: Uuid) -> Result<u64, AppError> {
        let result = sqlx::query("DELETE FROM api_tokens WHERE id = $1 AND user_id = $2")
            .bind(token_id)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}
