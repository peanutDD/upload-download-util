//! # API Token 服务模块
//!
//! 提供 API Token 的管理功能。
//!
//! ## 安全特性
//!
//! - 使用 HMAC-SHA256 加盐哈希，防止彩虹表攻击
//! - Token 只在创建时返回一次，数据库仅存储哈希值

use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    constants::API_TOKEN_CHARSET,
    models::api_token::{ApiToken, ApiTokenListItem, CreateApiTokenRequest},
    repositories::{api_tokens::CreateApiTokenRecord, ApiTokensRepo, FoldersRepo},
    utils::AppError,
};

type HmacSha256 = Hmac<Sha256>;

pub struct ApiTokenService {
    pool: PgPool,
    secrets: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ApiTokenClaims {
    pub token_id: Uuid,
    pub user_id: Uuid,
    pub webdav_enabled: bool,
    pub webdav_read_only: bool,
    pub webdav_root_folder_id: Option<Uuid>,
}

impl ApiTokenService {
    pub fn new(pool: PgPool, secrets: Vec<String>) -> Self {
        Self { pool, secrets }
    }

    /// 从 AppState 创建 ApiTokenService（工厂方法）
    pub fn from_state(state: &crate::AppState) -> Self {
        Self::new(
            state.pool.clone(),
            state.config.auth.api_token_hmac_secrets(),
        )
    }

    /// 生成安全的随机 token
    fn generate_token() -> String {
        use rand::Rng;
        let mut rng = rand::rng();
        (0..64)
            .map(|_| {
                let idx = rng.random_range(0..API_TOKEN_CHARSET.len());
                API_TOKEN_CHARSET[idx] as char
            })
            .collect()
    }

    /// 使用 HMAC-SHA256 加盐哈希 token
    ///
    /// 使用服务器密钥作为盐值，防止彩虹表攻击。secret 为空时返回错误，避免 panic。
    fn hash_token_with(secret: &str, token: &str) -> Result<String, AppError> {
        if secret.trim().is_empty() {
            return Err(AppError::Internal);
        }
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).map_err(|_| AppError::Internal)?;
        mac.update(token.as_bytes());
        let result = mac.finalize();
        Ok(format!("{:x}", result.into_bytes()))
    }

    fn hash_token(&self, token: &str) -> Result<String, AppError> {
        let Some(secret) = self.secrets.first() else {
            return Err(AppError::Internal);
        };
        Self::hash_token_with(secret, token)
    }

    /// 获取 token 前缀（用于显示）
    fn get_token_prefix(token: &str) -> String {
        token.chars().take(8).collect()
    }

    /// 创建新的 API token
    pub async fn create_token(
        &self,
        user_id: Uuid,
        req: CreateApiTokenRequest,
    ) -> Result<(String, ApiToken), AppError> {
        let repo = ApiTokensRepo::new(&self.pool);
        if let Some(root_folder_id) = req.webdav_root_folder_id {
            let folders = FoldersRepo::new(&self.pool);
            if !folders.exists(root_folder_id, user_id).await? {
                return Err(AppError::Validation(
                    "WebDAV root folder does not exist".to_string(),
                ));
            }
        }

        // 生成 token（只显示一次）
        let token = Self::generate_token();
        let token_hash = self.hash_token(&token)?;
        let token_prefix = Self::get_token_prefix(&token);

        // 计算过期时间
        let expires_at = req
            .expires_in_days
            .map(|days| Utc::now() + Duration::days(days as i64));

        // 创建 token
        let api_token = repo
            .create(CreateApiTokenRecord {
                user_id,
                name: &req.name,
                token_hash: &token_hash,
                token_prefix: &token_prefix,
                expires_at,
                webdav_enabled: req.webdav_enabled.unwrap_or(true),
                webdav_read_only: req.webdav_read_only.unwrap_or(false),
                webdav_root_folder_id: req.webdav_root_folder_id,
            })
            .await?;

        Ok((token, api_token))
    }

    /// 验证 token 并返回用户 ID
    pub async fn verify_token(&self, token: &str) -> Result<Uuid, AppError> {
        Ok(self.verify_token_claims(token).await?.user_id)
    }

    /// 验证 token 并返回 WebDAV/API scope claims
    pub async fn verify_token_claims(&self, token: &str) -> Result<ApiTokenClaims, AppError> {
        let repo = ApiTokensRepo::new(&self.pool);
        for secret in self.secrets.iter() {
            let token_hash = Self::hash_token_with(secret, token)?;
            if let Some(api_token) = repo.find_by_token_hash(&token_hash).await? {
                if let Some(exp) = api_token.expires_at {
                    if exp < Utc::now() {
                        return Err(AppError::Unauthorized);
                    }
                }
                repo.update_last_used(api_token.id).await?;
                return Ok(ApiTokenClaims {
                    token_id: api_token.id,
                    user_id: api_token.user_id,
                    webdav_enabled: api_token.webdav_enabled,
                    webdav_read_only: api_token.webdav_read_only,
                    webdav_root_folder_id: api_token.webdav_root_folder_id,
                });
            }
        }

        Err(AppError::Unauthorized)
    }

    /// 列出用户的所有 API token
    pub async fn list_tokens(&self, user_id: Uuid) -> Result<Vec<ApiTokenListItem>, AppError> {
        let repo = ApiTokensRepo::new(&self.pool);
        repo.list_by_user(user_id).await
    }

    /// 删除 API token
    pub async fn delete_token(&self, token_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        let repo = ApiTokensRepo::new(&self.pool);

        let affected = repo.delete(token_id, user_id).await?;

        if affected == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }
}
