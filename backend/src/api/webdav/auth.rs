use axum::http::{header, HeaderMap, StatusCode};
use base64::{engine::general_purpose::STANDARD, Engine as _};

use crate::{
    repositories::FoldersRepo,
    services::{
        api_token::{ApiTokenClaims, ApiTokenService},
        webdav::WebDavPrincipal,
    },
    AppState,
};

pub(super) async fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<WebDavPrincipal, StatusCode> {
    let Some(value) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let token_service = ApiTokenService::from_state(state);
    if let Some(encoded) = value.strip_prefix("Basic ") {
        let decoded = STANDARD
            .decode(encoded)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let Some((_, token)) = decoded.split_once(':') else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let claims = token_service
            .verify_token_claims(token)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        return principal_from_claims(state, claims).await;
    }
    if let Some(token) = value.strip_prefix("Bearer ") {
        let claims = token_service
            .verify_token_claims(token)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        return principal_from_claims(state, claims).await;
    }
    Err(StatusCode::UNAUTHORIZED)
}

async fn principal_from_claims(
    state: &AppState,
    claims: ApiTokenClaims,
) -> Result<WebDavPrincipal, StatusCode> {
    if !claims.webdav_enabled {
        return Err(StatusCode::FORBIDDEN);
    }
    if let Some(root_folder_id) = claims.webdav_root_folder_id {
        let exists = FoldersRepo::new(&state.pool)
            .exists(root_folder_id, claims.user_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !exists {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    Ok(WebDavPrincipal {
        api_token_id: claims.token_id,
        user_id: claims.user_id,
        read_only: claims.webdav_read_only,
        root_folder_id: claims.webdav_root_folder_id,
    })
}
