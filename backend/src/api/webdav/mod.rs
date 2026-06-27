use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderValue, Method, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use metrics::counter;

mod auth;
mod lock;
mod lock_headers;
mod methods;
mod path;
mod propfind;
mod range;
mod xml_fragments;

use self::{
    auth::authenticate,
    lock::{lock_path, unlock_path},
    methods::{copy_path, delete_path, get_file, mkcol, move_path, put_file},
    path::path_segments,
    propfind::propfind,
};

use crate::{services::webdav::WebDavError, AppState};

pub fn create_router() -> Router<AppState> {
    Router::new().fallback(any(handle_fallback))
}

pub async fn handle_root(State(state): State<AppState>, req: Request<Body>) -> Response {
    handle(state, req, String::new()).await
}

async fn handle_fallback(State(state): State<AppState>, req: Request<Body>) -> Response {
    let path = req
        .uri()
        .path()
        .strip_prefix("/dav")
        .unwrap_or(req.uri().path())
        .trim_start_matches('/')
        .to_string();
    handle(state, req, path).await
}

async fn handle(state: AppState, req: Request<Body>, raw_path: String) -> Response {
    let method = req.method().clone();
    let headers = req.headers().clone();
    let (parts, body) = req.into_parts();
    let segments = match path_segments(&raw_path) {
        Ok(segments) => segments,
        Err(status) => return status.into_response(),
    };
    let path_depth = segments.len();
    if method == Method::OPTIONS {
        return options_response();
    }

    let principal = match authenticate(&state, &headers).await {
        Ok(principal) => principal,
        Err(status) => {
            counter!("webdav_auth_fail_total").increment(1);
            let mut response = status.into_response();
            response.headers_mut().insert(
                header::WWW_AUTHENTICATE,
                HeaderValue::from_static("Basic realm=\"WebDAV\""),
            );
            return response;
        }
    };
    if principal.read_only && is_write_method(method.as_str()) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let response = match method.as_str() {
        "PROPFIND" => propfind(&state, &principal, &segments, &headers, body).await,
        "MKCOL" => mkcol(&state, &principal, &segments, body, &headers).await,
        "PUT" => put_file(&state, &principal, &segments, &headers, body).await,
        "GET" => get_file(&state, &principal, &segments, &headers, false).await,
        "HEAD" => get_file(&state, &principal, &segments, &headers, true).await,
        "DELETE" => delete_path(&state, &principal, &segments, &headers).await,
        "MOVE" => move_path(&state, &principal, &segments, &headers).await,
        "COPY" => copy_path(&state, &principal, &segments, &headers).await,
        "LOCK" => lock_path(&state, &principal, &segments, &headers, body).await,
        "UNLOCK" => unlock_path(&state, &principal, &headers).await,
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    };

    counter!(
        "webdav_request_total",
        "method" => method.as_str().to_string(),
        "status" => response.status().as_u16().to_string()
    )
    .increment(1);
    tracing::info!(
        method = %method,
        path_depth,
        user_id = %principal.user_id,
        status = response.status().as_u16(),
        "webdav request"
    );
    let _ = parts;
    response
}

fn is_write_method(method: &str) -> bool {
    matches!(
        method,
        "MKCOL" | "PUT" | "DELETE" | "MOVE" | "COPY" | "LOCK" | "UNLOCK"
    )
}

fn webdav_error_response(error: WebDavError) -> Response {
    match error {
        WebDavError::BadRequest => StatusCode::BAD_REQUEST,
        WebDavError::NotFound => StatusCode::NOT_FOUND,
        WebDavError::Conflict => StatusCode::CONFLICT,
        WebDavError::Forbidden => StatusCode::FORBIDDEN,
        WebDavError::PreconditionFailed => StatusCode::PRECONDITION_FAILED,
        WebDavError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
    }
    .into_response()
}

fn options_response() -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::ALLOW,
        HeaderValue::from_static(
            "OPTIONS, PROPFIND, MKCOL, PUT, GET, HEAD, DELETE, MOVE, COPY, LOCK, UNLOCK",
        ),
    );
    response
}
