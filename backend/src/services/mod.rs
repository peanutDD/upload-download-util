//! # 服务层模块
//!
//! 包含应用的核心业务逻辑，遵循以下设计原则：
//!
//! ## 设计原则
//!
//! 1. **单一职责**: 每个服务负责一个业务领域
//! 2. **依赖注入**: 通过构造函数注入依赖（数据库、配置等）
//! 3. **错误处理**: 使用 `Result<T, AppError>` 统一错误类型
//! 4. **事务安全**: 复杂操作使用数据库事务
//!
//! ## 服务列表
//!
//! - `auth`: 用户认证（注册、登录、密码管理）
//! - `file`: 文件管理（上传、下载、删除、分块上传）
//! - `folder`: 文件夹管理（创建、移动、删除、路径导航）
//! - `share`: 文件分享（创建、验证、访问）
//! - `api_token`: API Token 管理
//! - `storage`: 存储后端抽象（本地、S3）
//! - `cache`: 应用级缓存服务
//! - `organization`: 组织 / 多租户与权限服务

pub mod api_token;
pub mod auth;
pub mod cache;
pub mod embeddings;
pub mod file;
pub mod file_content_extractor;
pub mod folder;
pub mod fulltext_indexer;
pub mod fulltext_search;
pub mod maintenance;
pub mod ocr;
pub mod organization;
pub mod redis;
pub mod share;
pub mod storage;
pub mod task_queue;
pub mod webdav;
