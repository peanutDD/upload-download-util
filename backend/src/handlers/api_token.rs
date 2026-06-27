//! # API Token Handlers
//!
//! 处理 API Token 相关的 HTTP 请求，包括：
//! - 创建 API Token
//! - 列出用户的 API Token
//! - 删除 API Token
//!
//! ## 设计原则
//!
//! 1. **统一响应格式**: 使用 `utils::response` 中的辅助函数
//! 2. **业务逻辑分离**: 所有业务逻辑都在 `ApiTokenService` 中
//! 3. **输入验证**: 使用 `validator` crate 进行请求验证

use axum::extract::{Path, State};
use axum::response::Response;
use serde_json::json;
use uuid::Uuid;

use crate::extractors::AuthenticatedUser;
use crate::models::api_token::{ApiTokenResponse, CreateApiTokenRequest};
use crate::services::api_token::ApiTokenService;
use crate::utils::{json_response, success_response, AppError};
use crate::AppState;

/// 创建新的 API Token
///
/// # 请求体
/// ```json
/// {
///   "name": "My API Token",
///   "expires_in_days": 30
/// }
/// ```
///
/// # 响应
/// ```json
/// {
///   "token": {
///     "id": "...",
///     "name": "My API Token",
///     "token": "actual_token_string",
///     "expires_at": "...",
///     "created_at": "..."
///   }
/// }
/// ```
///
/// **注意**: Token 只在创建时返回一次，请妥善保存。
pub async fn create_token_handler(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    axum::Json(req): axum::Json<CreateApiTokenRequest>,
) -> Result<Response, AppError> {
    let token_service = ApiTokenService::from_state(&state);

    // 验证请求数据
    validator::Validate::validate(&req)
        .map_err(|e| AppError::Validation(format!("Validation error: {}", e)))?;

    // 创建 token
    let (token, api_token) = token_service.create_token(user_id, req).await?;

    Ok(json_response(json!({
        "token": ApiTokenResponse {
            id: api_token.id,
            name: api_token.name,
            token,
            expires_at: api_token.expires_at,
            created_at: api_token.created_at,
            webdav_enabled: api_token.webdav_enabled,
            webdav_read_only: api_token.webdav_read_only,
            webdav_root_folder_id: api_token.webdav_root_folder_id,
        }
    })))
}

/// 列出当前用户的所有 API Token
///
/// 返回用户创建的所有 API Token（不包含 token 值，仅元数据）。
pub async fn list_tokens_handler(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> Result<Response, AppError> {
    let token_service = ApiTokenService::from_state(&state);
    let tokens = token_service.list_tokens(user_id).await?;
    Ok(json_response(json!({ "tokens": tokens })))
}

/// 删除 API Token
///
/// 删除指定的 API Token，删除后该 token 将立即失效。
pub async fn delete_token_handler(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(token_id): Path<Uuid>,
) -> Result<Response, AppError> {
    let token_service = ApiTokenService::from_state(&state);
    token_service.delete_token(token_id, user_id).await?;
    Ok(success_response("Token deleted successfully"))
}
