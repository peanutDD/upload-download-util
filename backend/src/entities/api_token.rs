//! API Token 实体
//!
//! 对应数据库表 `api_tokens`。

use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct ApiToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub name: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub webdav_enabled: bool,
    pub webdav_read_only: bool,
    pub webdav_root_folder_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
