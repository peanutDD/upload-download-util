use axum::{
    body::{to_bytes, Body},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use futures::StreamExt;
use metrics::counter;
use tempfile::NamedTempFile;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_util::io::ReaderStream;

use super::{
    lock::{lock_conflicts, locked_response},
    path::{destination_segments, path_segments},
    range::parse_single_range,
    webdav_error_response,
};
use crate::{
    services::{
        storage::StorageReadStream,
        webdav::{WebDavError, WebDavMoveOutcome, WebDavPrincipal, WebDavPutInput, WebDavService},
    },
    utils::{effective_file_mime_type, is_macos_appledouble_filename},
    AppState,
};

pub(super) async fn mkcol(
    state: &AppState,
    principal: &WebDavPrincipal,
    segments: &[String],
    body: Body,
    headers: &HeaderMap,
) -> Response {
    let Ok(bytes) = to_bytes(body, 1).await else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if !bytes.is_empty() {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }
    if lock_conflicts(state, principal.user_id, segments, headers).await {
        return locked_response();
    }
    match WebDavService::new(state)
        .create_collection(principal, segments)
        .await
    {
        Ok(()) => StatusCode::CREATED.into_response(),
        Err(error) => webdav_error_response(error),
    }
}

pub(super) async fn put_file(
    state: &AppState,
    principal: &WebDavPrincipal,
    segments: &[String],
    headers: &HeaderMap,
    body: Body,
) -> Response {
    if lock_conflicts(state, principal.user_id, segments, headers).await {
        return locked_response();
    }
    let Some(filename) = segments.last().cloned() else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if is_macos_appledouble_filename(&filename) {
        tracing::info!(
            user_id = %principal.user_id,
            filename = %filename,
            "ignored macOS AppleDouble WebDAV upload"
        );
        return StatusCode::NO_CONTENT.into_response();
    }
    let mime_type = effective_file_mime_type(
        &filename,
        headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
    );
    let temp = match NamedTempFile::new() {
        Ok(file) => file,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let temp_path = temp.path().to_path_buf();
    let mut out = match tokio::fs::File::create(&temp_path).await {
        Ok(file) => file,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let mut stream = body.into_data_stream();
    let mut file_size = 0u64;
    while let Some(chunk) = stream.next().await {
        let Ok(chunk) = chunk else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        file_size = file_size.saturating_add(chunk.len() as u64);
        if file_size > state.config.storage.max_file_size {
            let _ = tokio::fs::remove_file(&temp_path).await;
            return StatusCode::PAYLOAD_TOO_LARGE.into_response();
        }
        if out.write_all(&chunk).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    if out.flush().await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    drop(out);
    let input = WebDavPutInput {
        mime_type,
        file_size,
        source_path: temp.path().to_path_buf(),
    };
    match WebDavService::new(state)
        .put_file_from_path(principal, segments, input)
        .await
    {
        Ok(_) => {
            counter!("webdav_bytes_written_total").increment(file_size);
            StatusCode::CREATED.into_response()
        }
        Err(error) => webdav_error_response(error),
    }
}

pub(super) async fn get_file(
    state: &AppState,
    principal: &WebDavPrincipal,
    segments: &[String],
    headers: &HeaderMap,
    head_only: bool,
) -> Response {
    let file = match WebDavService::new(state)
        .open_file_for_read(principal, segments, None, false)
        .await
    {
        Ok(read) => read.file,
        Err(error) => return webdav_error_response(error),
    };
    let total = file.file_size.max(0) as u64;
    let mut status = StatusCode::OK;
    let mut start = 0;
    let mut end = total.saturating_sub(1);
    if let Some(range) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        if let Some((s, e)) = parse_single_range(range, total) {
            status = StatusCode::PARTIAL_CONTENT;
            start = s;
            end = e;
        } else {
            return StatusCode::RANGE_NOT_SATISFIABLE.into_response();
        }
    }
    let len = if total == 0 { 0 } else { end - start + 1 };
    counter!("webdav_bytes_read_total").increment(len);
    let read = match WebDavService::new(state)
        .open_file_for_read(
            principal,
            segments,
            (status == StatusCode::PARTIAL_CONTENT).then_some((start, end)),
            !head_only,
        )
        .await
    {
        Ok(read) => read,
        Err(error) => {
            if error == WebDavError::NotFound {
                counter!("webdav_missing_storage_total").increment(1);
            }
            return webdav_error_response(error);
        }
    };
    let file = read.file;
    let mut response = if head_only || total == 0 {
        (status, Body::empty()).into_response()
    } else {
        let Some(stream) = read.stream else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };
        (status, stream_body(stream, start, len).await).into_response()
    };
    let headers = response.headers_mut();
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&file.mime_type)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&len.to_string()).unwrap(),
    );
    if status == StatusCode::PARTIAL_CONTENT {
        headers.insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{total}")).unwrap(),
        );
    }
    response
}

async fn stream_body(stream: StorageReadStream, start: u64, len: u64) -> Body {
    match stream {
        StorageReadStream::Local(mut file) => {
            if start > 0 {
                let _ = file.seek(std::io::SeekFrom::Start(start)).await;
            }
            Body::from_stream(ReaderStream::new(file.take(len)))
        }
        StorageReadStream::S3(stream) => {
            Body::from_stream(ReaderStream::new(stream.into_async_read()))
        }
    }
}

pub(super) async fn delete_path(
    state: &AppState,
    principal: &WebDavPrincipal,
    segments: &[String],
    headers: &HeaderMap,
) -> Response {
    if lock_conflicts(state, principal.user_id, segments, headers).await {
        return locked_response();
    }
    let service = WebDavService::new(state);
    match service.delete_path(principal, segments).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => webdav_error_response(error),
    }
}

pub(super) async fn move_path(
    state: &AppState,
    principal: &WebDavPrincipal,
    segments: &[String],
    headers: &HeaderMap,
) -> Response {
    if lock_conflicts(state, principal.user_id, segments, headers).await {
        return locked_response();
    }
    let Some(destination) = destination_segments(headers, segments) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(dest_segments) = path_segments(&destination) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if lock_conflicts(state, principal.user_id, &dest_segments, headers).await {
        return locked_response();
    }
    let service = WebDavService::new(state);
    match service
        .move_path(
            principal,
            segments,
            &dest_segments,
            overwrite_allowed(headers),
        )
        .await
    {
        Ok(WebDavMoveOutcome::Created) => StatusCode::CREATED.into_response(),
        Ok(WebDavMoveOutcome::NoContent) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => webdav_error_response(error),
    }
}

pub(super) async fn copy_path(
    state: &AppState,
    principal: &WebDavPrincipal,
    segments: &[String],
    headers: &HeaderMap,
) -> Response {
    if lock_conflicts(state, principal.user_id, segments, headers).await {
        return locked_response();
    }
    let Some(destination) = destination_segments(headers, segments) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(dest_segments) = path_segments(&destination) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if lock_conflicts(state, principal.user_id, &dest_segments, headers).await {
        return locked_response();
    }
    let service = WebDavService::new(state);
    match service
        .copy_path(
            principal,
            segments,
            &dest_segments,
            overwrite_allowed(headers),
        )
        .await
    {
        Ok(()) => StatusCode::CREATED.into_response(),
        Err(error) => webdav_error_response(error),
    }
}

fn overwrite_allowed(headers: &HeaderMap) -> bool {
    headers
        .get("overwrite")
        .and_then(|v| v.to_str().ok())
        .map(|value| !value.eq_ignore_ascii_case("F"))
        .unwrap_or(true)
}
