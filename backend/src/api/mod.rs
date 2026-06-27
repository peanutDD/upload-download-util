//! # API Module
//!
//! 路由定义模块，负责将 HTTP 请求路由到相应的处理器。
//!
//! ## 设计原则
//!
//! 1. **路由集中管理**: 所有路由定义都在此模块中
//! 2. **模块化路由**: 每个功能模块有独立的 `create_router()` 函数
//! 3. **清晰的路由结构**: 使用 RESTful 风格的路由命名

use axum::Router;

use crate::AppState;

pub mod admin;
pub mod api_token;
pub mod auth;
pub mod files;
pub mod folders;
pub mod oauth_github;
pub mod oauth_google;
pub mod openapi;
pub mod organizations;
pub mod proxy;
pub mod share;
pub mod telemetry;
pub mod webdav;

/// 返回 auth/files/folders/shares/tokens 聚合路由，供挂载到 `/api/v1` 与 `/api` 复用。
pub fn create_api_routes() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::create_router())
        .nest("/admin", admin::create_router())
        .nest("/org", organizations::create_router())
        .nest("/files", files::create_router())
        .nest("/folders", folders::create_router())
        .nest("/shares", share::create_router())
        .nest("/tokens", api_token::create_router())
        .nest("/telemetry", telemetry::create_router())
        .nest("/proxy", proxy::create_router())
}
