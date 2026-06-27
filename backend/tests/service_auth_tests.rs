//! # 认证服务层测试
//!
//! 测试 AuthService 的核心业务逻辑：
//! - 用户注册（happy path + error path）
//! - 用户登录（happy path + error path）
//! - JWT token 生成与验证
//! - Token 过期验证

mod common;

use common::{cleanup_test_data, create_test_pool, create_test_user, init_test_env};
use file_storage_backend::{
    config::Config,
    models::api_token::CreateApiTokenRequest,
    models::user::{LoginRequest, RegisterRequest, UpdateProfileRequest},
    repositories::{DynUsersRepo, SqlxUsersRepo},
    services::api_token::ApiTokenService,
    services::auth::{AuthService, AuthServiceError},
    services::cache::CacheService,
};
use serial_test::serial;
use std::sync::Arc;

// ============================================================================
// 测试辅助函数：创建测试 AuthService
// ============================================================================

async fn create_test_auth_service(pool: sqlx::PgPool) -> AuthService {
    let config = Config::from_env().unwrap_or_else(|_| Config::default_for_test());
    let users_repo: DynUsersRepo = Arc::new(SqlxUsersRepo::new(pool.clone()));
    let cache = CacheService::new();

    AuthService::new(users_repo, config, cache, None)
}

// ============================================================================
// 用户注册测试
// ============================================================================

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_register_happy_path() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let service = create_test_auth_service(pool.clone()).await;

    let req = RegisterRequest {
        username: "test_auth_register_happy".to_string(),
        email: "test_register@test.com".to_string(),
        password: "Password123!".to_string(),
    };

    let result = service.register(req).await;

    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user.username, "test_auth_register_happy");
    assert_eq!(user.email, "test_register@test.com");
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_register_duplicate_email() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let service = create_test_auth_service(pool.clone()).await;

    // 第一次注册成功
    let req1 = RegisterRequest {
        username: "test_auth_duplicate_1".to_string(),
        email: "same@test.com".to_string(),
        password: "Password123!".to_string(),
    };
    assert!(service.register(req1).await.is_ok());

    // 第二次使用相同邮箱注册应该失败
    let req2 = RegisterRequest {
        username: "test_auth_duplicate_2".to_string(),
        email: "same@test.com".to_string(),
        password: "Password123!".to_string(),
    };
    let result = service.register(req2).await;

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        AuthServiceError::Validation(_)
    ));
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_register_invalid_email() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let service = create_test_auth_service(pool.clone()).await;

    let req = RegisterRequest {
        username: "test_auth_invalid_email".to_string(),
        email: "invalid-email".to_string(),
        password: "Password123!".to_string(),
    };

    let result = service.register(req).await;

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        AuthServiceError::Validation(_)
    ));
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_register_short_password() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let service = create_test_auth_service(pool.clone()).await;

    let req = RegisterRequest {
        username: "test_auth_short_password".to_string(),
        email: "short_password@test.com".to_string(),
        password: "short".to_string(),
    };

    let result = service.register(req).await;

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        AuthServiceError::Validation(_)
    ));
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_register_empty_username() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let service = create_test_auth_service(pool.clone()).await;

    let req = RegisterRequest {
        username: "".to_string(),
        email: "empty_username@test.com".to_string(),
        password: "Password123!".to_string(),
    };

    let result = service.register(req).await;

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        AuthServiceError::Validation(_)
    ));
}

// ============================================================================
// 用户登录测试
// ============================================================================

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_login_happy_path() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    // 创建测试用户
    create_test_user(&pool, "login_test").await;

    let service = create_test_auth_service(pool.clone()).await;

    let req = LoginRequest {
        email: "test_login_test@test.com".to_string(),
        password: "test_password_123".to_string(),
    };

    let result = service.login(req).await;

    assert!(result.is_ok());
    let token = result.unwrap();
    assert!(!token.is_empty());
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_login_invalid_email() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let service = create_test_auth_service(pool.clone()).await;

    let req = LoginRequest {
        email: "nonexistent@test.com".to_string(),
        password: "Password123!".to_string(),
    };

    let result = service.login(req).await;

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        AuthServiceError::InvalidCredentials
    ));
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_login_wrong_password() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    // 创建测试用户
    create_test_user(&pool, "login_wrong_pwd").await;

    let service = create_test_auth_service(pool.clone()).await;

    let req = LoginRequest {
        email: "test_login_wrong_pwd@test.com".to_string(),
        password: "wrong_password".to_string(),
    };

    let result = service.login(req).await;

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        AuthServiceError::InvalidCredentials
    ));
}

// ============================================================================
// JWT Token 测试
// ============================================================================

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_generate_and_verify_token() {
    init_test_env();
    let pool = create_test_pool().await;

    let service = create_test_auth_service(pool.clone()).await;

    let user_id = uuid::Uuid::new_v4();

    // 生成 token
    let token = service.generate_token(&user_id).unwrap();
    assert!(!token.is_empty());

    // 验证 token
    let verified_id = service.verify_token(&token).unwrap();
    assert_eq!(verified_id, user_id);
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_verify_token_invalid_signature() {
    init_test_env();
    let pool = create_test_pool().await;

    let service = create_test_auth_service(pool.clone()).await;

    let invalid_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    let result = service.verify_token(invalid_token);

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        AuthServiceError::Unauthorized
    ));
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_verify_token_expired() {
    init_test_env();
    let pool = create_test_pool().await;

    let service = create_test_auth_service(pool.clone()).await;

    // 使用辅助函数创建过期的 token
    let config = Config::from_env().unwrap_or_else(|_| Config::default_for_test());
    let expired_token = create_expired_token(&config);

    let result = service.verify_token(&expired_token);

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        AuthServiceError::Unauthorized
    ));
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_verify_token_invalid_format() {
    init_test_env();
    let pool = create_test_pool().await;

    let service = create_test_auth_service(pool.clone()).await;

    let result = service.verify_token("not-a-valid-jwt-token");

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        AuthServiceError::Unauthorized
    ));
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_verify_token_empty() {
    init_test_env();
    let pool = create_test_pool().await;

    let service = create_test_auth_service(pool.clone()).await;

    let result = service.verify_token("");

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        AuthServiceError::Unauthorized
    ));
}

// ============================================================================
// 密码修改测试
// ============================================================================

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_change_password_happy_path() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (user_id, _, _) = create_test_user(&pool, "change_pwd").await;
    let service = create_test_auth_service(pool.clone()).await;

    let result = service
        .change_password(
            user_id,
            "test_password_123".to_string(),
            "NewPassword123!".to_string(),
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_change_password_wrong_current() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (user_id, _, _) = create_test_user(&pool, "change_pwd_wrong").await;
    let service = create_test_auth_service(pool.clone()).await;

    let result = service
        .change_password(
            user_id,
            "wrong_password".to_string(),
            "NewPassword123!".to_string(),
        )
        .await;

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        AuthServiceError::Validation(_)
    ));
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_change_password_weak_new() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (user_id, _, _) = create_test_user(&pool, "change_pwd_weak").await;
    let service = create_test_auth_service(pool.clone()).await;

    // 新密码太弱（没有数字）
    let result = service
        .change_password(
            user_id,
            "test_password_123".to_string(),
            "weakpassword".to_string(),
        )
        .await;

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        AuthServiceError::Validation(_)
    ));
}

// ============================================================================
// 用户资料更新测试
// ============================================================================

#[tokio::test]
#[serial(service_auth_db)]
async fn test_auth_service_update_profile_username_only() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (user_id, email, _) = create_test_user(&pool, "update_profile").await;
    let service = create_test_auth_service(pool.clone()).await;

    let req = UpdateProfileRequest {
        username: "new_username".to_string(),
        email: email.clone(),
        email_verification_code: None,
    };

    let result = service.update_profile(user_id, req).await;

    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user.username, "new_username");
    assert_eq!(user.email, email);
}

// ============================================================================
// API Token 服务测试
// ============================================================================

#[tokio::test]
#[serial(service_auth_db)]
async fn test_api_token_create_and_verify() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (user_id, _, _) = create_test_user(&pool, "api_token").await;

    let secrets = vec!["test_secret_key".to_string()];
    let token_service = ApiTokenService::new(pool.clone(), secrets);

    // 创建 token
    let req = CreateApiTokenRequest {
        name: "test-token".to_string(),
        expires_in_days: None,
        webdav_enabled: None,
        webdav_read_only: None,
        webdav_root_folder_id: None,
    };
    let (token, api_token) = token_service.create_token(user_id, req).await.unwrap();

    assert!(!token.is_empty());
    assert_eq!(api_token.name, "test-token");
    assert_eq!(api_token.user_id, user_id);

    // 验证 token
    let verified_user_id = token_service.verify_token(&token).await.unwrap();
    assert_eq!(verified_user_id, user_id);
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_api_token_verify_invalid() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let secrets = vec!["test_secret_key".to_string()];
    let token_service = ApiTokenService::new(pool.clone(), secrets);

    let result = token_service.verify_token("invalid-token-12345").await;

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        file_storage_backend::utils::AppError::Unauthorized
    ));
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_api_token_list() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (user_id, _, _) = create_test_user(&pool, "api_token_list").await;

    let secrets = vec!["test_secret_key".to_string()];
    let token_service = ApiTokenService::new(pool.clone(), secrets);

    // 创建多个 token
    let req1 = CreateApiTokenRequest {
        name: "token-1".to_string(),
        expires_in_days: None,
        webdav_enabled: None,
        webdav_read_only: None,
        webdav_root_folder_id: None,
    };
    let req2 = CreateApiTokenRequest {
        name: "token-2".to_string(),
        expires_in_days: None,
        webdav_enabled: None,
        webdav_read_only: None,
        webdav_root_folder_id: None,
    };
    token_service.create_token(user_id, req1).await.unwrap();
    token_service.create_token(user_id, req2).await.unwrap();

    // 列出 token
    let tokens = token_service.list_tokens(user_id).await.unwrap();
    assert_eq!(tokens.len(), 2);
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_api_token_delete() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (user_id, _, _) = create_test_user(&pool, "api_token_delete").await;

    let secrets = vec!["test_secret_key".to_string()];
    let token_service = ApiTokenService::new(pool.clone(), secrets);

    // 创建 token
    let req = CreateApiTokenRequest {
        name: "to-delete".to_string(),
        expires_in_days: None,
        webdav_enabled: None,
        webdav_read_only: None,
        webdav_root_folder_id: None,
    };
    let (_, api_token) = token_service.create_token(user_id, req).await.unwrap();

    // 删除 token
    let result = token_service.delete_token(api_token.id, user_id).await;
    assert!(result.is_ok());

    // 验证已删除
    let tokens = token_service.list_tokens(user_id).await.unwrap();
    assert_eq!(tokens.len(), 0);
}

#[tokio::test]
#[serial(service_auth_db)]
async fn test_api_token_delete_nonexistent() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (user_id, _, _) = create_test_user(&pool, "api_token_delete_nonexistent").await;

    let secrets = vec!["test_secret_key".to_string()];
    let token_service = ApiTokenService::new(pool.clone(), secrets);

    let fake_token_id = uuid::Uuid::new_v4();
    let result = token_service.delete_token(fake_token_id, user_id).await;

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap(),
        file_storage_backend::utils::AppError::NotFound
    ));
}

// ============================================================================
// 测试辅助函数
// ============================================================================

fn create_expired_token(config: &Config) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
        iat: usize,
    }

    let claims = Claims {
        sub: uuid::Uuid::new_v4().to_string(),
        exp: 1000000000, // 过去的时间（2001年）
        iat: 999999999,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.auth.jwt_secret.as_ref()),
    )
    .unwrap()
}
