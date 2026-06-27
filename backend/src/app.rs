//! # 应用构建模块
//!
//! 负责创建并配置 Axum 应用：CORS、中间件栈、路由挂载。
//! 与 `main.rs` 中的启动流程解耦，便于测试与阅读。

use std::time::Duration;

use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::Router;
use tower::ServiceBuilder;
use tower::{limit::ConcurrencyLimitLayer, load_shed::LoadShedLayer, BoxError};
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{AllowOrigin, Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use crate::api;
use crate::config::Config;
use crate::handlers::health;
use crate::middleware;
use crate::utils::error_response;
use crate::AppState;

// ---------- 应用创建 ----------

/// 创建并配置 Axum 应用
///
/// # 参数
/// - `app_state`: 应用共享状态
/// - `config`: 应用配置（CORS、速率限制等）
/// - `metrics_renderer`: Prometheus metrics 渲染器
///
/// # 返回
/// 配置完成的 Axum Router（未 bind，由 main 负责 serve）
pub async fn create_app<F>(app_state: AppState, config: &Config, metrics_renderer: F) -> Router
where
    F: Fn() -> String + Clone + Send + Sync + 'static,
{
    let cors = create_cors_layer(config);
    // 全局 IP 级限流 + 已登录用户写操作 user 级限流（可通过 IP_RATE_LIMIT、USER_RATE_LIMIT 等环境变量调整）
    let rate_limit_state = middleware::rate_limit::create_rate_limit_middleware(
        config.rate_limit.ip_rate_limit,
        config.rate_limit.user_rate_limit,
        config.rate_limit.rate_limit_window_secs,
        config.rate_limit.rate_limit_max_keys,
        app_state.redis.clone(),
    );

    // Router::layer 要求 L::Service: Clone。RequestLogLayer 及其 Service 已实现 Clone。
    let middleware_stack = ServiceBuilder::new()
        .layer(axum::error_handling::HandleErrorLayer::new(
            |err: BoxError| async move {
                tracing::warn!("global overload triggered: {}", err);
                // 过载保护也应保持与 AppError 一致的错误结构，便于客户端统一解析错误响应。
                error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "SERVICE_OVERLOADED",
                    "服务器繁忙，请稍后重试",
                )
            },
        ))
        .layer(LoadShedLayer::new())
        .layer(ConcurrencyLimitLayer::new(512))
        .layer(CatchPanicLayer::custom(middleware::panic::JsonPanicHandler))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::request_log::RequestLogLayer)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(120),
        ))
        .layer(axum::middleware::from_fn(webdav_compat_headers))
        .layer(cors)
        .into_inner();

    let metrics_handler = {
        let renderer = metrics_renderer.clone();
        move || {
            let r = renderer.clone();
            async move { r() }
        }
    };

    // 按 Axum 文档：from_fn_with_state 用 route_layer 在 with_state 之前挂载。
    // 请求流向：middleware_stack（含 RequestLogLayer）-> metrics -> rate_limit -> router
    let routes = Router::new()
        .route("/health", get(health::health_check))
        .route("/livez", get(health::liveness_check))
        .route("/metrics", get(metrics_handler))
        .route("/readyz", get(health::readiness_check))
        .merge(api::openapi::create_openapi_router())
        .route("/dav/", any(api::webdav::handle_root))
        .route("/dav", any(api::webdav::handle_root))
        .nest("/dav", api::webdav::create_router())
        .nest("/api/v1", api::create_api_routes())
        .nest("/api", api::create_api_routes())
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            move |axum::extract::State(app_state_for_mw): axum::extract::State<AppState>,
                  req,
                  next| {
                let limit_state = rate_limit_state.clone();
                middleware::rate_limit::rate_limit_middleware(
                    app_state_for_mw,
                    limit_state,
                    req,
                    next,
                )
            },
        ))
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            middleware::metrics::metrics_middleware,
        ));

    routes.with_state(app_state).layer(middleware_stack)
}

// ---------- CORS ----------

async fn webdav_compat_headers(req: Request<axum::body::Body>, next: Next) -> Response {
    let is_webdav = req.uri().path().starts_with("/dav");
    if is_webdav && req.method() == axum::http::Method::OPTIONS {
        return add_webdav_headers(StatusCode::NO_CONTENT.into_response());
    }
    let mut response = next.run(req).await;
    if is_webdav {
        response = add_webdav_headers(response);
    }
    response
}

fn add_webdav_headers(mut response: Response) -> Response {
    response.headers_mut().insert(
        axum::http::HeaderName::from_static("dav"),
        axum::http::HeaderValue::from_static("1, 2"),
    );
    response.headers_mut().insert(
        axum::http::HeaderName::from_static("ms-author-via"),
        axum::http::HeaderValue::from_static("DAV"),
    );
    response.headers_mut().insert(
        axum::http::header::ALLOW,
        axum::http::HeaderValue::from_static(
            "OPTIONS, PROPFIND, MKCOL, PUT, GET, HEAD, DELETE, MOVE, COPY, LOCK, UNLOCK",
        ),
    );
    response
}

/// 根据配置创建 CORS 中间件层
fn create_cors_layer(config: &Config) -> CorsLayer {
    let (cors_origin, allow_credentials): (AllowOrigin, bool) = {
        let raw = config.server.cors_origin.trim();
        if raw == "*" {
            (Any.into(), false)
        } else {
            let mut origins: Vec<axum::http::HeaderValue> = raw
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .filter_map(|s| s.parse::<axum::http::HeaderValue>().ok())
                .collect();
            for extra in [
                "tauri://localhost",
                "http://tauri.localhost",
                "https://tauri.localhost",
            ] {
                if let Ok(v) = extra.parse::<axum::http::HeaderValue>() {
                    if !origins.contains(&v) {
                        origins.push(v);
                    }
                }
            }
            if origins.is_empty() {
                (Any.into(), false)
            } else {
                (AllowOrigin::list(origins), true)
            }
        }
    };

    CorsLayer::new()
        .allow_origin(cors_origin)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::DELETE,
            axum::http::Method::PUT,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
            axum::http::header::ORIGIN,
            axum::http::header::RANGE,
            axum::http::HeaderName::from_static("x-part-sha256"),
        ])
        .allow_credentials(allow_credentials)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{
            header::{
                ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_REQUEST_HEADERS,
                ACCESS_CONTROL_REQUEST_METHOD, ORIGIN,
            },
            Method, Request,
        },
        routing::put,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn cors_preflight_allows_chunked_part_checksum_header() {
        let mut config = Config::default_for_test();
        config.server.cors_origin = "http://192.168.0.108:5173".to_string();

        let app = Router::new()
            .route(
                "/api/files/upload/chunked/{id}/chunk",
                put(|| async { StatusCode::OK }),
            )
            .layer(create_cors_layer(&config));

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/api/files/upload/chunked/test-upload/chunk?part=1")
                    .header(ORIGIN, "http://192.168.0.108:5173")
                    .header(ACCESS_CONTROL_REQUEST_METHOD, "PUT")
                    .header(
                        ACCESS_CONTROL_REQUEST_HEADERS,
                        "x-part-sha256, authorization, content-type",
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let allowed_headers = response
            .headers()
            .get(ACCESS_CONTROL_ALLOW_HEADERS)
            .expect("preflight should return allowed headers")
            .to_str()
            .unwrap()
            .to_ascii_lowercase();

        assert!(
            allowed_headers
                .split(',')
                .any(|header| header.trim() == "x-part-sha256"),
            "allowed headers should include x-part-sha256, got: {allowed_headers}"
        );
    }
}
