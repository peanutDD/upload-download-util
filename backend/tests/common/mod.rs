//! # 测试公共模块
//!
//! 提供测试辅助函数和测试环境设置。

#![allow(dead_code)]

pub mod app;

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::Once;
use std::sync::OnceLock;
use url::Url;

static INIT: Once = Once::new();
static MIGRATIONS_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

/// 初始化测试环境
///
/// 设置日志和环境变量
pub fn init_test_env() {
    INIT.call_once(|| {
        // 设置测试环境
        std::env::set_var("RUST_LOG", "debug");

        // 初始化 tracing（测试时）
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_env_filter("debug")
            .try_init();
    });
}

/// 创建测试数据库连接池
///
/// 使用 DATABASE_URL 环境变量或默认测试数据库
pub async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        let user = std::env::var("PGUSER")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "postgres".to_string());
        let password = std::env::var("PGPASSWORD").ok();
        match password {
            Some(p) if !p.is_empty() => {
                format!("postgres://{}:{}@localhost:5432/file_storage_test", user, p)
            }
            _ => format!("postgres://{}@localhost:5432/file_storage_test", user),
        }
    });

    ensure_test_database(&database_url).await;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create test database pool");

    let lock = MIGRATIONS_LOCK.get_or_init(|| tokio::sync::Mutex::new(()));
    let _guard = lock.lock().await;
    file_storage_backend::database::pool::pre_migration_repairs(&pool)
        .await
        .expect("Failed to run pre-migration repairs");
    let migrator = sqlx::migrate!("./migrations");
    if let Err(err) = migrator.run(&pool).await {
        if matches!(err, sqlx::migrate::MigrateError::VersionMismatch(_)) {
            sqlx::query("DROP SCHEMA public CASCADE")
                .execute(&pool)
                .await
                .expect("Failed to reset test database schema (drop public)");
            sqlx::query("CREATE SCHEMA public")
                .execute(&pool)
                .await
                .expect("Failed to reset test database schema (create public)");

            file_storage_backend::database::pool::pre_migration_repairs(&pool)
                .await
                .expect("Failed to run pre-migration repairs");
            migrator
                .run(&pool)
                .await
                .expect("Failed to run test database migrations after schema reset");
        } else {
            panic!("Failed to run test database migrations: {}", err);
        }
    }

    pool
}

async fn ensure_test_database(database_url: &str) {
    let mut url = Url::parse(database_url).expect("Invalid DATABASE_URL");
    let db_name = url.path().trim_start_matches('/').to_string();
    if db_name.is_empty() {
        return;
    }
    if !db_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        panic!("Invalid database name");
    }

    url.set_path("/postgres");
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(url.as_str())
        .await
        .expect("Failed to connect to postgres database");

    let exists: Option<i32> = sqlx::query_scalar("SELECT 1 FROM pg_database WHERE datname = $1")
        .bind(&db_name)
        .fetch_optional(&admin_pool)
        .await
        .expect("Failed to check database existence");

    if exists.is_none() {
        let create_sql = format!("CREATE DATABASE \"{}\"", db_name);
        if let Err(err) = sqlx::query(&create_sql).execute(&admin_pool).await {
            let should_ignore = err
                .as_database_error()
                .and_then(|db_err| db_err.code().map(|code| code == "42P04" || code == "23505"))
                .unwrap_or(false);
            if !should_ignore {
                panic!("Failed to create test database: {}", err);
            }
        }
    }
}

/// 清理测试数据
///
/// 清空测试相关的数据表
pub async fn cleanup_test_data(pool: &PgPool) {
    // 按照外键依赖顺序清理
    let _ = sqlx::query("DELETE FROM file_shares").execute(pool).await;
    let _ = sqlx::query("DELETE FROM background_tasks")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM files").execute(pool).await;
    let _ = sqlx::query("DELETE FROM folders").execute(pool).await;
    let _ = sqlx::query("DELETE FROM upload_sessions")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM webdav_locks").execute(pool).await;
    let _ = sqlx::query("DELETE FROM api_tokens").execute(pool).await;
    let _ = sqlx::query("DELETE FROM users WHERE email LIKE '%@test.com'")
        .execute(pool)
        .await;
}

/// 创建测试用户
///
/// 返回 (user_id, email, password)
#[allow(dead_code)]
pub async fn create_test_user(pool: &PgPool, suffix: &str) -> (uuid::Uuid, String, String) {
    let email = format!("test_{}@test.com", suffix);
    let username = format!("test_user_{}", suffix);
    let password = "test_password_123";

    // 使用 bcrypt 哈希密码
    let password_hash =
        bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash password");

    let user_id: (uuid::Uuid,) = sqlx::query_as(
        "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(&username)
    .bind(&email)
    .bind(&password_hash)
    .fetch_one(pool)
    .await
    .expect("Failed to create test user");

    (user_id.0, email, password.to_string())
}

/// 创建测试文件记录
#[allow(dead_code)]
pub async fn create_test_file(pool: &PgPool, user_id: uuid::Uuid, filename: &str) -> uuid::Uuid {
    let file_id = uuid::Uuid::new_v4();
    let config = file_storage_backend::config::Config::from_env()
        .unwrap_or_else(|_| file_storage_backend::config::Config::default_for_test());
    let file_path = std::path::PathBuf::from(config.storage.path)
        .join("test-fixtures")
        .join(user_id.to_string())
        .join(file_id.to_string())
        .join(filename);

    if let Some(parent) = file_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .expect("Failed to create test file parent directory");
    }
    tokio::fs::write(&file_path, vec![b'x'; 1024])
        .await
        .expect("Failed to write test file fixture");
    let file_path = file_path.to_string_lossy().to_string();

    sqlx::query(
        "INSERT INTO files (id, user_id, filename, original_filename, file_path, file_size, mime_type, storage_backend) \
         VALUES ($1, $2, $3, $3, $4, 1024, 'application/octet-stream', 'local')"
    )
    .bind(file_id)
    .bind(user_id)
    .bind(filename)
    .bind(&file_path)
    .execute(pool)
    .await
    .expect("Failed to create test file");

    file_id
}

/// 创建测试文件夹
#[allow(dead_code)]
pub async fn create_test_folder(
    pool: &PgPool,
    user_id: uuid::Uuid,
    name: &str,
    parent_id: Option<uuid::Uuid>,
) -> uuid::Uuid {
    let folder_id: (uuid::Uuid,) = sqlx::query_as(
        "INSERT INTO folders (user_id, name, parent_id) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(user_id)
    .bind(name)
    .bind(parent_id)
    .fetch_one(pool)
    .await
    .expect("Failed to create test folder");

    folder_id.0
}
