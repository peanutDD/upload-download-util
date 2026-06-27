use axum::{
    body::{to_bytes, Body},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use metrics::counter;
use quick_xml::{events::Event, reader::Reader, writer::Writer};
use std::collections::HashMap;
use uuid::Uuid;

use super::{
    lock_headers::{header_lock_token, request_lock_tokens},
    path::lock_key,
    propfind::xml_local_name,
    xml_fragments::{escape_xml, xml_element, xml_empty},
};
use crate::{services::webdav::WebDavPrincipal, AppState};

const DEFAULT_LOCK_TIMEOUT_SECS: i64 = 600;
const MAX_LOCK_TIMEOUT_SECS: i64 = 3600;

pub(super) type LockDiscoveryByPath = HashMap<String, String>;
type LockConflictRow = (String, String, String);

pub(super) async fn lock_path(
    state: &AppState,
    principal: &WebDavPrincipal,
    segments: &[String],
    headers: &HeaderMap,
    body: Body,
) -> Response {
    let owner = match request_lock_owner(body).await {
        Ok(owner) => owner,
        Err(status) => return status.into_response(),
    };
    let path = lock_key(segments);
    if lock_conflicts(state, principal.user_id, segments, headers).await {
        return locked_response();
    }
    let token = format!("opaquelocktoken:{}", Uuid::new_v4());
    let depth = headers
        .get("depth")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("0")
        .to_string();
    let timeout_secs = parse_timeout_secs(headers);
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(timeout_secs);
    let refresh_tokens = request_lock_tokens(headers);
    for request_token in &refresh_tokens {
        let refreshed = sqlx::query_as::<_, (String, Option<String>)>(
            r#"
            UPDATE webdav_locks
            SET depth = $1, owner = COALESCE($2::text, owner), expires_at = $3, updated_at = CURRENT_TIMESTAMP
            WHERE user_id = $4
              AND api_token_id = $5
              AND path = $6
              AND token = $7
              AND expires_at > NOW()
            RETURNING token, owner
            "#,
        )
        .bind(&depth)
        .bind(owner.as_deref())
        .bind(expires_at)
        .bind(principal.user_id)
        .bind(principal.api_token_id)
        .bind(&path)
        .bind(request_token)
        .fetch_optional(&state.pool)
        .await;
        match refreshed {
            Ok(Some((token, owner))) => {
                return lock_response(&depth, timeout_secs, &token, owner.as_deref());
            }
            Ok(None) => {}
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
    if !refresh_tokens.is_empty() {
        return locked_response();
    }
    if sqlx::query(
        "INSERT INTO webdav_locks (user_id, api_token_id, path, token, depth, owner, expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(principal.user_id)
    .bind(principal.api_token_id)
    .bind(&path)
    .bind(&token)
    .bind(&depth)
    .bind(owner.as_deref())
    .bind(expires_at)
    .execute(&state.pool)
    .await
    .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    lock_response(&depth, timeout_secs, &token, owner.as_deref())
}

async fn request_lock_owner(body: Body) -> Result<Option<String>, StatusCode> {
    let bytes = to_bytes(body, 64 * 1024)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    if bytes.is_empty() {
        return Ok(None);
    }
    let xml = std::str::from_utf8(&bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    parse_lock_owner(xml).map_err(|_| StatusCode::BAD_REQUEST)
}

fn parse_lock_owner(xml: &str) -> Result<Option<String>, quick_xml::Error> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut writer = Writer::new(Vec::new());
    let mut depth = 0usize;
    let mut owner_depth = None;

    loop {
        match reader.read_event()? {
            Event::Start(event) => {
                let is_owner = xml_local_name(event.local_name().as_ref()) == "owner";
                if owner_depth.is_some() {
                    writer.write_event(Event::Start(event.into_owned()))?;
                } else if is_owner {
                    owner_depth = Some(depth);
                }
                depth += 1;
            }
            Event::Empty(event) => {
                let is_owner = xml_local_name(event.local_name().as_ref()) == "owner";
                if owner_depth.is_some() {
                    writer.write_event(Event::Empty(event.into_owned()))?;
                } else if is_owner {
                    return Ok(Some(String::new()));
                }
            }
            Event::Text(event) if owner_depth.is_some() => {
                writer.write_event(Event::Text(event.into_owned()))?;
            }
            Event::CData(event) if owner_depth.is_some() => {
                writer.write_event(Event::CData(event.into_owned()))?;
            }
            Event::End(event) => {
                depth = depth.saturating_sub(1);
                if owner_depth == Some(depth) {
                    return Ok(Some(
                        String::from_utf8(writer.into_inner()).unwrap_or_default(),
                    ));
                }
                if owner_depth.is_some() {
                    writer.write_event(Event::End(event.into_owned()))?;
                }
            }
            Event::Eof => return Ok(None),
            _ => {}
        }
    }
}

pub(super) async fn active_lock_discoveries(
    state: &AppState,
    user_id: Uuid,
) -> Result<LockDiscoveryByPath, StatusCode> {
    let rows: Vec<(String, String, String, Option<String>, i64)> = sqlx::query_as(
        r#"
        SELECT path,
               token,
               depth,
               owner,
               GREATEST(1, CEIL(EXTRACT(EPOCH FROM (expires_at - NOW()))))::BIGINT AS timeout_secs
        FROM webdav_locks
        WHERE user_id = $1
          AND expires_at > NOW()
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(rows
        .into_iter()
        .map(|(path, token, depth, owner, timeout_secs)| {
            (
                path,
                active_lock_xml(&depth, timeout_secs, &token, owner.as_deref()),
            )
        })
        .collect())
}

fn active_lock_xml(depth: &str, timeout_secs: i64, token: &str, owner: Option<&str>) -> String {
    let mut active_lock = format!(
        r#"<D:activelock><D:locktype><D:write/></D:locktype><D:lockscope><D:exclusive/></D:lockscope><D:depth>{}</D:depth>"#,
        escape_xml(depth)
    );
    if let Some(owner) = owner {
        if owner.is_empty() {
            active_lock.push_str(&xml_empty("D:owner"));
        } else {
            active_lock.push_str(&xml_element("D:owner", owner));
        }
    }
    active_lock.push_str(&format!(
        r#"<D:timeout>Second-{}</D:timeout><D:locktoken><D:href>{}</D:href></D:locktoken></D:activelock>"#,
        timeout_secs,
        escape_xml(token)
    ));
    xml_element("D:lockdiscovery", &active_lock)
}

fn lock_response(depth: &str, timeout_secs: i64, token: &str, owner: Option<&str>) -> Response {
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><D:prop xmlns:D="DAV:">{}</D:prop>"#,
        active_lock_xml(depth, timeout_secs, token, owner)
    );
    let mut response = (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        body,
    )
        .into_response();
    response.headers_mut().insert(
        axum::http::HeaderName::from_static("lock-token"),
        HeaderValue::from_str(&format!("<{token}>")).unwrap(),
    );
    response
}

pub(super) async fn unlock_path(
    state: &AppState,
    principal: &WebDavPrincipal,
    headers: &HeaderMap,
) -> Response {
    let Some(token) = header_lock_token(headers) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    match sqlx::query(
        "DELETE FROM webdav_locks WHERE user_id = $1 AND api_token_id = $2 AND token = $3",
    )
    .bind(principal.user_id)
    .bind(principal.api_token_id)
    .bind(token)
    .execute(&state.pool)
    .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub(super) fn locked_response() -> Response {
    StatusCode::from_u16(423).unwrap().into_response()
}

fn parse_timeout_secs(headers: &HeaderMap) -> i64 {
    headers
        .get("timeout")
        .and_then(|v| v.to_str().ok())
        .and_then(|value| {
            value
                .split(',')
                .find_map(|part| part.trim().strip_prefix("Second-"))
        })
        .and_then(|seconds| seconds.parse::<i64>().ok())
        .map(|seconds| seconds.clamp(1, MAX_LOCK_TIMEOUT_SECS))
        .unwrap_or(DEFAULT_LOCK_TIMEOUT_SECS)
}

pub(super) async fn lock_conflicts(
    state: &AppState,
    user_id: Uuid,
    segments: &[String],
    headers: &HeaderMap,
) -> bool {
    let request_tokens = request_lock_tokens(headers);
    let path = lock_key(segments);
    let rows = sqlx::query_as::<_, LockConflictRow>(
        r#"
        SELECT path, token, depth
        FROM webdav_locks
        WHERE user_id = $1
          AND expires_at > NOW()
          AND (
              path = $2
              OR starts_with(path, $2 || '/')
              OR (lower(depth) = 'infinity' AND starts_with($2, path || '/'))
          )
        "#,
    )
    .bind(user_id)
    .bind(&path)
    .fetch_all(&state.pool)
    .await;
    let conflict = lock_rows_have_conflict(rows, &request_tokens, &path);
    if conflict {
        counter!("webdav_lock_conflict_total").increment(1);
    }
    conflict
}

fn lock_rows_have_conflict(
    rows: Result<Vec<LockConflictRow>, sqlx::Error>,
    request_tokens: &[String],
    request_path: &str,
) -> bool {
    match rows {
        Ok(rows) => rows.into_iter().any(|(locked_path, token, depth)| {
            lock_applies_to_path(&locked_path, &depth, request_path)
                && !request_tokens.iter().any(|t| t == &token)
        }),
        Err(error) => {
            tracing::error!(
                error = %error,
                request_path,
                "webdav lock lookup failed; failing closed"
            );
            counter!("webdav_lock_lookup_error_total").increment(1);
            true
        }
    }
}

fn lock_applies_to_path(locked_path: &str, depth: &str, request_path: &str) -> bool {
    locked_path == request_path
        || locked_path.starts_with(&format!("{request_path}/"))
        || (depth.eq_ignore_ascii_case("infinity")
            && request_path.starts_with(&format!("{locked_path}/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_lookup_errors_fail_closed() {
        let request_tokens = Vec::new();

        assert!(lock_rows_have_conflict(
            Err(sqlx::Error::RowNotFound),
            &request_tokens,
            "/locked.txt"
        ));
    }
}
