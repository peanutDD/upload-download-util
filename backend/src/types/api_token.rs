//! API Token 相关 DTO

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateApiTokenRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub expires_in_days: Option<u32>,
    pub webdav_enabled: Option<bool>,
    pub webdav_read_only: Option<bool>,
    pub webdav_root_folder_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct ApiTokenResponse {
    pub id: Uuid,
    pub name: String,
    pub token: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub webdav_enabled: bool,
    pub webdav_read_only: bool,
    pub webdav_root_folder_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct ApiTokenListItem {
    pub id: Uuid,
    pub name: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub webdav_enabled: bool,
    pub webdav_read_only: bool,
    pub webdav_root_folder_id: Option<Uuid>,
}
