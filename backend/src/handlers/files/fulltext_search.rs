use axum::extract::{Query, State};
use axum::response::Response;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    extractors::auth::AuthenticatedUser,
    models::file::FileResponse,
    services::{
        fulltext_search::{SearchDocument, SearchHit},
        ocr::OcrExtractor,
    },
    utils::{json_response, AppError},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct FulltextSearchQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub folder_id: Option<Uuid>,
    pub mime_type: Option<String>,
}

fn default_limit() -> usize {
    20
}

pub async fn fulltext_search_handler(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(params): Query<FulltextSearchQuery>,
) -> Result<Response, AppError> {
    let query = params.q.trim();
    if query.is_empty() {
        return Err(AppError::Validation("查询文本不能为空".to_string()));
    }
    if params.limit == 0 || params.limit > 100 {
        return Err(AppError::Validation(
            "结果数量限制必须在 1 到 100 之间".to_string(),
        ));
    }
    if !state.config.search.fulltext_search_enabled {
        return Err(AppError::File("全文搜索功能未启用".to_string()));
    }

    let folder_prefix = params.folder_id.map(|id| format!("/{id}/"));
    let mut hits = state.search_index.search(
        user_id,
        query,
        params.limit,
        folder_prefix.as_deref(),
        params.mime_type.as_deref(),
    )?;
    let mut index_status = "ready";

    if hits.is_empty() {
        index_status = "fallback";
        hits = fallback_search(
            &state,
            user_id,
            query,
            params.limit,
            params.folder_id,
            params.mime_type.as_deref(),
        )
        .await?;
    }

    let files = materialize_hits(&state, user_id, hits).await?;

    Ok(json_response(json!({
        "query": query,
        "count": files.len(),
        "index_status": index_status,
        "files": files,
    })))
}

pub async fn ocr_status_handler(
    State(state): State<AppState>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> Result<Response, AppError> {
    Ok(json_response(json!({
        "enabled": state.config.search.ocr_enabled,
        "pdf_max_pages": state.config.search.ocr_pdf_max_pages,
        "tesseract": {
            "bin": state.config.search.ocr_tesseract_bin,
            "available": OcrExtractor::command_available(&state.config.search.ocr_tesseract_bin),
        },
        "poppler": {
            "bin": state.config.search.ocr_pdftoppm_bin,
            "available": OcrExtractor::command_available(&state.config.search.ocr_pdftoppm_bin),
        },
    })))
}

async fn materialize_hits(
    state: &AppState,
    user_id: Uuid,
    hits: Vec<SearchHit>,
) -> Result<Vec<serde_json::Value>, AppError> {
    let mut files = Vec::with_capacity(hits.len());
    for hit in hits {
        let file = match state.file_service.get_file(hit.file_id, user_id).await {
            Ok(file) => FileResponse::from(file),
            Err(AppError::NotFound) => continue,
            Err(error) => return Err(error),
        };
        files.push(json!({
            "file": file,
            "score": hit.score,
            "snippet": hit.snippet,
            "match_source": hit.match_source,
        }));
    }
    Ok(files)
}

async fn fallback_search(
    state: &AppState,
    user_id: Uuid,
    query: &str,
    limit: usize,
    folder_id: Option<Uuid>,
    mime_type: Option<&str>,
) -> Result<Vec<SearchHit>, AppError> {
    let files = state
        .file_service
        .list_by_folder(user_id, folder_id)
        .await?;
    let needle = query.to_lowercase();
    let mut hits = Vec::new();
    for file in files.into_iter() {
        if let Some(filter) = mime_type {
            if file.mime_type != filter {
                continue;
            }
        }
        if !file.original_filename.to_lowercase().contains(&needle) {
            continue;
        }
        hits.push(SearchHit {
            file_id: file.id,
            filename: file.original_filename.clone(),
            path: format!("/{}", file.original_filename),
            mime_type: file.mime_type.clone(),
            score: 0.1,
            snippet: file.original_filename.chars().take(160).collect(),
            match_source: "filename".to_string(),
        });
        if hits.len() >= limit {
            break;
        }
    }
    Ok(hits)
}

#[allow(dead_code)]
pub(crate) fn document_from_file(
    file: &crate::models::file::File,
    extracted_text: String,
) -> SearchDocument {
    SearchDocument {
        file_id: file.id,
        user_id: file.user_id,
        filename: file.original_filename.clone(),
        path: format!("/{}", file.original_filename),
        extracted_text,
        ocr_text: String::new(),
        category: file.category.clone().unwrap_or_default(),
        mime_type: file.mime_type.clone(),
    }
}
