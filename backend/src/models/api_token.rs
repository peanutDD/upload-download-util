//! 兼容转发层
//!
//! - DB 实体 → [`crate::entities::api_token`] ✅ 已迁移
//! - API DTO → [`crate::types::api_token`] ✅ 已迁移

pub use crate::entities::api_token::ApiToken;

pub use crate::types::api_token::{ApiTokenListItem, ApiTokenResponse, CreateApiTokenRequest};

impl From<crate::entities::api_token::ApiToken> for ApiTokenListItem {
    fn from(token: crate::entities::api_token::ApiToken) -> Self {
        ApiTokenListItem {
            id: token.id,
            name: token.name,
            last_used_at: token.last_used_at,
            expires_at: token.expires_at,
            created_at: token.created_at,
            webdav_enabled: token.webdav_enabled,
            webdav_read_only: token.webdav_read_only,
            webdav_root_folder_id: token.webdav_root_folder_id,
        }
    }
}
