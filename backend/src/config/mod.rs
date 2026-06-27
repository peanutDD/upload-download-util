use serde::Deserialize;
use std::env;
use std::path::PathBuf;
use thiserror::Error;

pub mod auth;
pub mod cache;
pub mod database;
pub mod oauth;
pub mod rate_limit;
pub mod redis;
pub mod search;
pub mod server;
pub mod storage;
pub mod tasks;

pub use auth::AuthConfig;
pub use cache::CacheConfig;
pub use database::DatabaseConfig;
pub use oauth::OAuthConfig;
pub use rate_limit::RateLimitConfig;
pub use redis::RedisConfig;
pub use search::SearchConfig;
pub use server::ServerConfig;
pub use storage::{HlsAbrVariant, StorageConfig};
pub use tasks::TasksConfig;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub redis: Option<RedisConfig>,
    pub auth: AuthConfig,
    pub storage: StorageConfig,
    pub server: ServerConfig,
    pub tasks: TasksConfig,
    pub rate_limit: RateLimitConfig,
    pub oauth: OAuthConfig,
    pub search: SearchConfig,
    pub cache: CacheConfig,
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Config loading error: {0}")]
    LoadError(#[from] config::ConfigError),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        // 环境映射
        let mut builder = config::Config::builder()
            // 默认值设置
            .set_default("server.port", "3000")?
            .set_default("server.cors_origin", "*")?
            .set_default("server.trust_proxy_headers", "false")?
            .set_default("storage.backend", "local")?
            .set_default("storage.download_mode", "proxy")?
            .set_default("storage.max_file_size", "2147483648")?
            .set_default("storage.presign_ttl_secs", "300")?
            .set_default("storage.hls_threshold_bytes", "104857600")?
            .set_default("storage.hls_abr_max_variants", "1")?
            .set_default("storage.aws_region", "us-east-1")?
            .set_default("tasks.queue_backend", "postgres")?
            .set_default("tasks.upload_session_cleanup_interval_secs", "300")?
            .set_default("tasks.upload_session_cleanup_batch_size", "200")?
            .set_default("tasks.files_consistency_check_interval_secs", "600")?
            .set_default("tasks.files_consistency_check_batch_size", "500")?
            .set_default("tasks.orphan_cleanup_interval_secs", "600")?
            .set_default("tasks.orphan_cleanup_batch_limit", "500")?
            .set_default("tasks.transcode_max_concurrent", "2")?
            .set_default("tasks.zip_cache_enabled", "false")?
            .set_default("tasks.zip_cache_backend", "local")?
            .set_default("tasks.zip_cache_ttl_secs", "3600")?
            .set_default("tasks.zip_build_max_concurrent", "2")?
            .set_default("rate_limit.ip_rate_limit", "600")?
            .set_default("rate_limit.user_rate_limit", "600")?
            .set_default("rate_limit.rate_limit_window_secs", "60")?
            .set_default("rate_limit.rate_limit_max_keys", "20000")?
            .set_default(
                "search.huggingface_model_id",
                "sentence-transformers/all-MiniLM-L6-v2",
            )?
            .set_default(
                "search.huggingface_api_url",
                "https://api-inference.huggingface.co",
            )?
            .set_default("search.fulltext_search_enabled", "true")?
            .set_default("search.fulltext_index_path", "search-index")?
            .set_default("search.ocr_enabled", "false")?
            .set_default("search.ocr_tesseract_bin", "tesseract")?
            .set_default("search.ocr_pdftoppm_bin", "pdftoppm")?
            .set_default("search.ocr_pdf_max_pages", 5)?
            .set_default("cache.enabled", "true")?
            .set_default("cache.default_ttl_secs", "60")?
            .set_default("cache.list_ttl_secs", "20")?
            .set_default("auth.jwt_expiry", "24h")?
            .add_source(config::Environment::default().separator("__")); // 支持嵌套结构如 DATABASE__URL

        // 传统平铺环境变量兼容映射
        let aliases = [
            ("DATABASE_URL", "database.url"),
            ("READ_REPLICA_DATABASE_URL", "database.read_replica_url"),
            ("REDIS_URL", "redis.url"),
            ("JWT_SECRET", "auth.jwt_secret"),
            ("JWT_EXPIRY", "auth.jwt_expiry"),
            ("API_TOKEN_HMAC_SECRET", "auth.api_token_hmac_secret"),
            (
                "API_TOKEN_HMAC_SECRET_PREVIOUS",
                "auth.api_token_hmac_secret_previous",
            ),
            ("ADMIN_TOKEN", "auth.admin_token"),
            ("PORT", "server.port"),
            ("CORS_ORIGIN", "server.cors_origin"),
            ("FRONTEND_BASE_URL", "server.frontend_base_url"),
            ("TRUST_PROXY_HEADERS", "server.trust_proxy_headers"),
            ("STORAGE_BACKEND", "storage.backend"),
            ("STORAGE_PATH", "storage.path"),
            ("MAX_FILE_SIZE", "storage.max_file_size"),
            ("ALLOWED_MIME_TYPES", "storage.allowed_mime_types"),
            ("DOWNLOAD_MODE", "storage.download_mode"),
            ("PRESIGN_TTL_SECS", "storage.presign_ttl_secs"),
            ("AWS_ACCESS_KEY_ID", "storage.aws_access_key_id"),
            ("AWS_SECRET_ACCESS_KEY", "storage.aws_secret_access_key"),
            ("AWS_REGION", "storage.aws_region"),
            ("AWS_BUCKET", "storage.aws_bucket"),
            ("HLS_THRESHOLD_BYTES", "storage.hls_threshold_bytes"),
            ("HLS_ABR_MAX_VARIANTS", "storage.hls_abr_max_variants"),
            ("TASK_QUEUE_BACKEND", "tasks.queue_backend"),
            (
                "UPLOAD_SESSION_CLEANUP_INTERVAL_SECS",
                "tasks.upload_session_cleanup_interval_secs",
            ),
            (
                "UPLOAD_SESSION_CLEANUP_BATCH_SIZE",
                "tasks.upload_session_cleanup_batch_size",
            ),
            (
                "FILES_CONSISTENCY_CHECK_INTERVAL_SECS",
                "tasks.files_consistency_check_interval_secs",
            ),
            (
                "FILES_CONSISTENCY_CHECK_BATCH_SIZE",
                "tasks.files_consistency_check_batch_size",
            ),
            (
                "ORPHAN_CLEANUP_INTERVAL_SECS",
                "tasks.orphan_cleanup_interval_secs",
            ),
            (
                "ORPHAN_CLEANUP_BATCH_LIMIT",
                "tasks.orphan_cleanup_batch_limit",
            ),
            ("TRANSCODE_MAX_CONCURRENT", "tasks.transcode_max_concurrent"),
            ("ZIP_CACHE_ENABLED", "tasks.zip_cache_enabled"),
            ("ZIP_CACHE_BACKEND", "tasks.zip_cache_backend"),
            ("ZIP_CACHE_TTL_SECS", "tasks.zip_cache_ttl_secs"),
            ("ZIP_BUILD_MAX_CONCURRENT", "tasks.zip_build_max_concurrent"),
            ("IP_RATE_LIMIT", "rate_limit.ip_rate_limit"),
            ("USER_RATE_LIMIT", "rate_limit.user_rate_limit"),
            (
                "RATE_LIMIT_WINDOW_SECS",
                "rate_limit.rate_limit_window_secs",
            ),
            ("RATE_LIMIT_MAX_KEYS", "rate_limit.rate_limit_max_keys"),
            ("GITHUB_CLIENT_ID", "oauth.github_client_id"),
            ("GITHUB_CLIENT_SECRET", "oauth.github_client_secret"),
            (
                "GITHUB_OAUTH_REDIRECT_URI",
                "oauth.github_oauth_redirect_uri",
            ),
            ("GOOGLE_CLIENT_ID", "oauth.google_client_id"),
            ("GOOGLE_CLIENT_SECRET", "oauth.google_client_secret"),
            (
                "GOOGLE_OAUTH_REDIRECT_URI",
                "oauth.google_oauth_redirect_uri",
            ),
            ("HUGGINGFACE_API_TOKEN", "search.huggingface_api_token"),
            ("HUGGINGFACE_MODEL_ID", "search.huggingface_model_id"),
            ("HUGGINGFACE_API_URL", "search.huggingface_api_url"),
            ("FULLTEXT_SEARCH_ENABLED", "search.fulltext_search_enabled"),
            ("SEARCH_INDEX_PATH", "search.fulltext_index_path"),
            ("OCR_ENABLED", "search.ocr_enabled"),
            ("OCR_TESSERACT_BIN", "search.ocr_tesseract_bin"),
            ("OCR_PDFTOPPM_BIN", "search.ocr_pdftoppm_bin"),
            ("OCR_PDF_MAX_PAGES", "search.ocr_pdf_max_pages"),
            ("CACHE_ENABLED", "cache.enabled"),
            ("CACHE_DEFAULT_TTL_SECS", "cache.default_ttl_secs"),
            ("LIST_CACHE_TTL_SECS", "cache.list_ttl_secs"),
            ("SMTP_HOST", "server.smtp_host"),
            ("SMTP_PORT", "server.smtp_port"),
            ("SMTP_USERNAME", "server.smtp_username"),
            ("SMTP_PASSWORD", "server.smtp_password"),
            ("SMTP_FROM", "server.smtp_from"),
        ];

        for (env_name, config_path) in aliases {
            if let Ok(val) = env::var(env_name) {
                builder = builder.set_override(config_path, val)?;
            }
        }

        let s = builder.build()?;
        let mut config: Config = s.try_deserialize()?;

        // 特殊处理：默认存储路径
        if config.storage.path.is_empty() {
            let default_storage_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
                .join("uploads")
                .to_string_lossy()
                .to_string();
            config.storage.path = default_storage_path;
        }

        // 特殊处理：HLS 变体解析 (如果环境变量存在且不是默认值，可能需要手动解析，
        // 但 config crate 默认不支持从字符串解析为 Vec<Struct>，除非是 JSON)
        // 这里为了简化，如果 HLS_ABR_VARIANTS 存在，我们手动补齐
        if let Ok(raw) = env::var("HLS_ABR_VARIANTS") {
            config.storage.hls_abr_variants = parse_hls_abr_variants_from_str(&raw)?;
        } else if config.storage.hls_abr_variants.is_empty() {
            config.storage.hls_abr_variants =
                parse_hls_abr_variants_from_str("240:350,360:700,480:1200,720:2500")?;
        }

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.server.port == 0 {
            return Err(ConfigError::InvalidConfig("PORT must be > 0".into()));
        }
        if self.database.url.is_empty() {
            return Err(ConfigError::MissingEnvVar("DATABASE_URL".into()));
        }
        if self.auth.jwt_secret.is_empty() {
            return Err(ConfigError::MissingEnvVar("JWT_SECRET".into()));
        }
        // ... 其他校验逻辑 ...
        Ok(())
    }

    /// 创建测试用的默认配置
    ///
    /// 用于集成测试，提供合理的默认值
    pub fn default_for_test() -> Self {
        Self {
            database: DatabaseConfig {
                url: "postgres://postgres@localhost:5432/file_storage_test".to_string(),
                read_replica_url: None,
            },
            redis: None,
            auth: AuthConfig {
                jwt_secret: "test_jwt_secret_that_is_long_enough_for_hs256".to_string(),
                jwt_expiry: "24h".to_string(),
                api_token_hmac_secret: Some("test_hmac_secret".to_string()),
                api_token_hmac_secret_previous: None,
                admin_token: None,
            },
            storage: StorageConfig {
                backend: "local".to_string(),
                path: "/tmp/test_uploads".to_string(),
                max_file_size: 2147483648,
                allowed_mime_types: vec![],
                download_mode: "proxy".to_string(),
                presign_ttl_secs: 300,
                aws_access_key_id: None,
                aws_secret_access_key: None,
                aws_region: Some("us-east-1".to_string()),
                aws_bucket: None,
                hls_threshold_bytes: 104857600,
                hls_abr_max_variants: 1,
                hls_abr_variants: vec![],
            },
            server: ServerConfig {
                port: 3000,
                cors_origin: "*".to_string(),
                frontend_base_url: Some("http://localhost:5173".to_string()),
                trust_proxy_headers: false,
                smtp_host: None,
                smtp_port: None,
                smtp_username: None,
                smtp_password: None,
                smtp_from: None,
            },
            tasks: TasksConfig {
                queue_backend: "postgres".to_string(),
                upload_session_cleanup_interval_secs: 300,
                upload_session_cleanup_batch_size: 200,
                files_consistency_check_interval_secs: 600,
                files_consistency_check_batch_size: 500,
                orphan_cleanup_interval_secs: 600,
                orphan_cleanup_batch_limit: 500,
                transcode_max_concurrent: 2,
                task_type_concurrency: std::collections::HashMap::new(),
                zip_cache_enabled: false,
                zip_cache_backend: "local".to_string(),
                zip_cache_ttl_secs: 3600,
                zip_build_max_concurrent: 2,
            },
            rate_limit: RateLimitConfig {
                ip_rate_limit: 600,
                user_rate_limit: 600,
                rate_limit_window_secs: 60,
                rate_limit_max_keys: 20000,
            },
            oauth: OAuthConfig {
                github_client_id: None,
                github_client_secret: None,
                github_oauth_redirect_uri: None,
                google_client_id: None,
                google_client_secret: None,
                google_oauth_redirect_uri: None,
            },
            search: SearchConfig {
                huggingface_api_token: None,
                huggingface_model_id: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
                huggingface_api_url: "https://api-inference.huggingface.co".to_string(),
                fulltext_search_enabled: true,
                fulltext_index_path: std::env::temp_dir()
                    .join(format!(
                        "file-storage-test-search-index-{}",
                        uuid::Uuid::new_v4()
                    ))
                    .to_string_lossy()
                    .to_string(),
                ocr_enabled: false,
                ocr_tesseract_bin: "tesseract".to_string(),
                ocr_pdftoppm_bin: "pdftoppm".to_string(),
                ocr_pdf_max_pages: 5,
            },
            cache: CacheConfig {
                enabled: true,
                default_ttl_secs: 60,
                list_ttl_secs: 20,
            },
        }
    }
}

fn parse_hls_abr_variants_from_str(raw: &str) -> Result<Vec<HlsAbrVariant>, ConfigError> {
    let mut out = Vec::new();
    for part in raw.split(',') {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        let Some((h, br)) = t.split_once(':') else {
            return Err(ConfigError::InvalidConfig(
                "HLS_ABR_VARIANTS must be height:bitrate".into(),
            ));
        };
        out.push(HlsAbrVariant {
            height: h
                .trim()
                .parse()
                .map_err(|_| ConfigError::InvalidConfig("Invalid height".into()))?,
            video_bitrate_kbps: br
                .trim()
                .parse()
                .map_err(|_| ConfigError::InvalidConfig("Invalid bitrate".into()))?,
        });
    }
    Ok(out)
}
