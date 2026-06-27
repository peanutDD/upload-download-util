mod common;

use std::{fs, sync::Arc};

use axum::{
    body::{to_bytes, Body as AxumBody},
    http::StatusCode,
    Router,
};
use file_storage_backend::{
    app::create_app,
    config::Config,
    services::{
        fulltext_search::{SearchDocument, SearchIndexService},
        ocr::{OcrExtractor, OcrOptions, OcrStatus},
        storage::create_memory_backend,
        task_queue::{run_fulltext_index_worker, run_fulltext_remove_worker},
    },
    AppState,
};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

#[test]
fn fulltext_search_returns_snippets_and_enforces_user_isolation() {
    let service = SearchIndexService::open_in_memory().unwrap();
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();
    service
        .upsert_document(SearchDocument {
            file_id: Uuid::new_v4(),
            user_id: user_a,
            filename: "notes.md".into(),
            path: "/notes.md".into(),
            extracted_text: "rust webdav searchable content".into(),
            ocr_text: String::new(),
            category: String::new(),
            mime_type: "text/markdown".into(),
        })
        .unwrap();
    service
        .upsert_document(SearchDocument {
            file_id: Uuid::new_v4(),
            user_id: user_b,
            filename: "secret.md".into(),
            path: "/secret.md".into(),
            extracted_text: "rust webdav searchable content".into(),
            ocr_text: String::new(),
            category: String::new(),
            mime_type: "text/markdown".into(),
        })
        .unwrap();

    let results = service.search(user_a, "webdav", 10, None, None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].filename, "notes.md");
    assert!(results[0].snippet.to_lowercase().contains("webdav"));
}

#[test]
fn fulltext_search_marks_ocr_text_as_ocr_source() {
    let service = SearchIndexService::open_in_memory().unwrap();
    let user_id = Uuid::new_v4();
    service
        .upsert_document(SearchDocument {
            file_id: Uuid::new_v4(),
            user_id,
            filename: "scan.png".into(),
            path: "/scan.png".into(),
            extracted_text: String::new(),
            ocr_text: "invoice number zebra-445".into(),
            category: String::new(),
            mime_type: "image/png".into(),
        })
        .unwrap();

    let results = service
        .search(user_id, "zebra-445", 10, None, None)
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].match_source, "ocr");
}

#[test]
fn ocr_extractor_skips_when_disabled_or_dependency_missing() {
    let disabled = OcrExtractor::extract(&[], "image/png", "scan.png", false, "tesseract").unwrap();
    assert_eq!(disabled.status, OcrStatus::Disabled);
    assert!(disabled.text.is_empty());

    let missing = OcrExtractor::extract(
        b"not a real image",
        "image/png",
        "scan.png",
        true,
        "/definitely/missing/tesseract",
    )
    .unwrap();
    assert_eq!(missing.status, OcrStatus::DependencyMissing);
    assert!(missing.text.is_empty());

    let unsupported =
        OcrExtractor::extract(b"plain text", "text/plain", "note.txt", true, "tesseract").unwrap();
    assert_eq!(unsupported.status, OcrStatus::Unsupported);
    assert!(unsupported.text.is_empty());
}

#[test]
fn backend_docker_image_installs_ocr_runtime_dependencies() {
    let dockerfile = fs::read_to_string("Dockerfile").unwrap();

    assert!(dockerfile.contains("tesseract-ocr"));
    assert!(dockerfile.contains("poppler-utils"));
}

#[test]
fn pdf_ocr_converts_limited_pages_then_indexes_each_page_text() {
    let temp = tempfile::tempdir().unwrap();
    let pdftoppm = temp.path().join("pdftoppm");
    let tesseract = temp.path().join("tesseract");
    let args_file = temp.path().join("pdftoppm.args");

    fs::write(
        &pdftoppm,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nfor last do :; done\nprintf page1 > \"${{last}}-1.png\"\nprintf page2 > \"${{last}}-2.png\"\n",
            args_file.display()
        ),
    )
    .unwrap();
    fs::write(
        &tesseract,
        "#!/bin/sh\necho \"ocr text from $(basename \"$1\")\"\n",
    )
    .unwrap();
    make_executable(&pdftoppm);
    make_executable(&tesseract);

    let outcome = OcrExtractor::extract_with_options(
        b"%PDF fake scan",
        "application/pdf",
        "scan.pdf",
        OcrOptions {
            enabled: true,
            tesseract_bin: tesseract.to_string_lossy().to_string(),
            pdftoppm_bin: pdftoppm.to_string_lossy().to_string(),
            pdf_max_pages: 2,
        },
    )
    .unwrap();

    assert_eq!(outcome.status, OcrStatus::Completed);
    assert!(outcome.text.contains("ocr text from page-1.png"));
    assert!(outcome.text.contains("ocr text from page-2.png"));
    let args = fs::read_to_string(args_file).unwrap();
    assert!(args.contains("-l\n2"));
}

#[test]
fn pdf_ocr_skips_when_poppler_dependency_is_missing() {
    let outcome = OcrExtractor::extract_with_options(
        b"%PDF fake scan",
        "application/pdf",
        "scan.pdf",
        OcrOptions {
            enabled: true,
            tesseract_bin: "tesseract".to_string(),
            pdftoppm_bin: "/definitely/missing/pdftoppm".to_string(),
            pdf_max_pages: 2,
        },
    )
    .unwrap();

    assert_eq!(outcome.status, OcrStatus::DependencyMissing);
    assert!(outcome.text.is_empty());
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) {}

#[tokio::test]
#[serial(fulltext_search_db)]
async fn fulltext_api_returns_files_contract_from_persistent_index() {
    common::init_test_env();
    let pool = common::create_test_pool().await;
    common::cleanup_test_data(&pool).await;

    let index_dir = tempfile::tempdir().unwrap();
    let (app, state) = build_fulltext_app(&pool, index_dir.path(), false).await;
    let (user_id, token) = common::app::login_and_get_token(&pool, "fulltext_api_contract").await;
    let file_id = common::create_test_file(&pool, user_id, "research.md").await;
    state
        .search_index
        .upsert_document(SearchDocument {
            file_id,
            user_id,
            filename: "research.md".into(),
            path: "/research.md".into(),
            extracted_text: "tantivy persistent content contract".into(),
            ocr_text: String::new(),
            category: String::new(),
            mime_type: "text/markdown".into(),
        })
        .unwrap();

    let json = search_fulltext(app, &token, "persistent").await;

    assert_eq!(json["query"], "persistent");
    assert_eq!(json["count"], 1);
    assert!(json.get("results").is_none());
    assert_eq!(json["files"][0]["file"]["id"], file_id.to_string());
    assert_eq!(json["files"][0]["file"]["file_size"], 1024);
    assert_eq!(json["files"][0]["match_source"], "content");
    assert!(json["files"][0]["snippet"]
        .as_str()
        .unwrap()
        .contains("persistent"));
}

#[tokio::test]
#[serial(fulltext_search_db)]
async fn upload_worker_indexes_content_for_fulltext_api() {
    common::init_test_env();
    let pool = common::create_test_pool().await;
    common::cleanup_test_data(&pool).await;

    let index_dir = tempfile::tempdir().unwrap();
    let (app, state) = build_fulltext_app(&pool, index_dir.path(), false).await;
    let (_user_id, token) = common::app::login_and_get_token(&pool, "fulltext_upload").await;
    let file_id = upload_text(
        app.clone(),
        &token,
        "worker-note.txt",
        "Worker should index narwhal-persistent searchable text.",
    )
    .await;

    let queued: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM background_tasks WHERE task_type = 'search_index_file' AND dedupe_key = $1")
            .bind(format!("search:{file_id}"))
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(queued.0, 1);

    run_fulltext_index_worker(&state).await.unwrap();

    let json = search_fulltext(app, &token, "narwhal-persistent").await;
    assert_eq!(json["count"], 1);
    assert_eq!(json["files"][0]["file"]["id"], file_id.to_string());
    assert_eq!(json["files"][0]["match_source"], "content");
}

#[tokio::test]
#[serial(fulltext_search_db)]
async fn delete_worker_removes_file_from_fulltext_index() {
    common::init_test_env();
    let pool = common::create_test_pool().await;
    common::cleanup_test_data(&pool).await;

    let index_dir = tempfile::tempdir().unwrap();
    let (app, state) = build_fulltext_app(&pool, index_dir.path(), false).await;
    let (user_id, token) = common::app::login_and_get_token(&pool, "fulltext_delete").await;
    let file_id = upload_text(
        app,
        &token,
        "delete-note.txt",
        "Delete worker removes orchid-searchable text.",
    )
    .await;
    run_fulltext_index_worker(&state).await.unwrap();
    assert_eq!(
        state
            .search_index
            .search(user_id, "orchid-searchable", 10, None, None)
            .unwrap()
            .len(),
        1
    );

    state
        .file_service
        .delete_file(file_id, user_id)
        .await
        .unwrap();
    run_fulltext_remove_worker(&state).await.unwrap();

    assert!(state
        .search_index
        .search(user_id, "orchid-searchable", 10, None, None)
        .unwrap()
        .is_empty());
}

#[tokio::test]
#[serial(fulltext_search_db)]
async fn fulltext_worker_completes_when_ocr_dependency_is_missing() {
    common::init_test_env();
    let pool = common::create_test_pool().await;
    common::cleanup_test_data(&pool).await;

    let index_dir = tempfile::tempdir().unwrap();
    let (app, state) = build_fulltext_app(&pool, index_dir.path(), true).await;
    let (_user_id, token) = common::app::login_and_get_token(&pool, "fulltext_ocr_missing").await;
    let file_id = upload_file(
        app.clone(),
        &token,
        "scan.png",
        "image/png",
        b"not a real image",
    )
    .await;

    run_fulltext_index_worker(&state).await.unwrap();

    let task: (String, Option<String>) =
        sqlx::query_as("SELECT status, last_error FROM background_tasks WHERE task_type = 'search_index_file' AND dedupe_key = $1")
            .bind(format!("search:{file_id}"))
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(task.0, "succeeded");
    assert!(task.1.is_none());

    let json = search_fulltext(app, &token, "scan").await;
    assert_eq!(json["count"], 1);
    assert_eq!(json["files"][0]["file"]["id"], file_id.to_string());
    assert_eq!(json["files"][0]["match_source"], "filename");
}

#[tokio::test]
#[serial(fulltext_search_db)]
async fn ocr_status_endpoint_reports_runtime_dependencies() {
    common::init_test_env();
    let pool = common::create_test_pool().await;
    common::cleanup_test_data(&pool).await;

    let index_dir = tempfile::tempdir().unwrap();
    let (app, _state) = build_fulltext_app(&pool, index_dir.path(), true).await;
    let (_user_id, token) = common::app::login_and_get_token(&pool, "ocr_status").await;
    let (auth_name, auth_value) = common::app::bearer_auth_header(&token);

    let response = app
        .oneshot(
            axum::http::Request::get("/api/v1/files/search/ocr/status")
                .header(auth_name, auth_value)
                .body(AxumBody::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["enabled"], true);
    assert_eq!(json["tesseract"]["available"], false);
    assert_eq!(json["poppler"]["available"], false);
    assert_eq!(json["pdf_max_pages"], 5);
}

async fn build_fulltext_app(
    pool: &sqlx::PgPool,
    index_path: &std::path::Path,
    ocr_enabled: bool,
) -> (Router, AppState) {
    let mut config = Config::default_for_test();
    config.search.fulltext_index_path = index_path.to_string_lossy().to_string();
    config.search.ocr_enabled = ocr_enabled;
    config.search.ocr_tesseract_bin = "/definitely/missing/tesseract".to_string();
    config.search.ocr_pdftoppm_bin = "/definitely/missing/pdftoppm".to_string();
    let config = Arc::new(config);
    let storage = Arc::new(create_memory_backend());
    let state = AppState::new(config.clone(), pool.clone(), pool.clone(), storage, None);
    let app = create_app(state.clone(), config.as_ref(), || "".to_string()).await;
    (app, state)
}

async fn upload_text(app: Router, token: &str, filename: &str, text: &str) -> Uuid {
    upload_file(app, token, filename, "text/plain", text.as_bytes()).await
}

async fn upload_file(
    app: Router,
    token: &str,
    filename: &str,
    content_type: &str,
    data: &[u8],
) -> Uuid {
    let boundary = "fulltext-boundary";
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let (auth_name, auth_value) = common::app::bearer_auth_header(token);
    let response = app
        .oneshot(
            axum::http::Request::post("/api/v1/files/upload")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header(auth_name, auth_value)
                .body(AxumBody::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    Uuid::parse_str(json["file"]["id"].as_str().unwrap()).unwrap()
}

async fn search_fulltext(app: Router, token: &str, query: &str) -> serde_json::Value {
    let (auth_name, auth_value) = common::app::bearer_auth_header(token);
    let response = app
        .oneshot(
            axum::http::Request::get(format!("/api/v1/files/search/fulltext?q={query}"))
                .header(auth_name, auth_value)
                .body(AxumBody::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}
