//! # 文件上传 Handler 集成测试
//!
//! 测试 upload / instant / chunked 相关的 HTTP endpoint。
//! 使用真 Router + 真 PG（遵循约束 C-004）。

mod common;

use axum::{
    body::{to_bytes, Body as AxumBody},
    http::StatusCode,
};
use bytes::Bytes;
use common::{
    app::{bearer_auth_header, build_test_app, login_and_get_token},
    cleanup_test_data, create_test_pool, init_test_env,
};
use file_storage_backend::utils::sha256_hex;
use tower::ServiceExt;

use serial_test::serial;

// ============================================================================
// 普通上传测试 (POST /upload)
// ============================================================================

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_upload_file_handler_happy_path() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "upload_happy").await;
    let app = build_test_app(&pool).await;

    // 构建 multipart 请求
    let boundary = "test-boundary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\nContent-Type: text/plain\r\n\r\nHello World\r\n--{boundary}--\r\n"
    );

    let (auth_name, auth_value) = bearer_auth_header(&token);
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
}

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_upload_file_handler_infers_previewable_mime_from_filename() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "upload_infer_mime").await;
    let app = build_test_app(&pool).await;

    let boundary = "test-boundary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"finder-image.png\"\r\nContent-Type: application/octet-stream\r\n\r\npng-bytes\r\n--{boundary}--\r\n"
    );

    let (auth_name, auth_value) = bearer_auth_header(&token);
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
    assert_eq!(json["file"]["mime_type"], "image/png");
}

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_upload_file_handler_no_file_field() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "upload_nofile").await;
    let app = build_test_app(&pool).await;

    let boundary = "test-boundary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"other\"; filename=\"test.txt\"\r\nContent-Type: text/plain\r\n\r\nHello World\r\n--{boundary}--\r\n"
    );

    let (auth_name, auth_value) = bearer_auth_header(&token);
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

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_upload_file_handler_invalid_folder_id() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "upload_invalid_folder").await;
    let app = build_test_app(&pool).await;

    let boundary = "test-boundary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"folder_id\"\r\n\r\ninvalid-uuid\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\nContent-Type: text/plain\r\n\r\nHello World\r\n--{boundary}--\r\n"
    );

    let (auth_name, auth_value) = bearer_auth_header(&token);
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

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_upload_file_handler_rejects_foreign_folder_id() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_owner_id, token) = login_and_get_token(&pool, "upload_foreign_owner").await;
    let (other_user_id, _, _) = common::create_test_user(&pool, "upload_foreign_other").await;
    let foreign_folder_id =
        common::create_test_folder(&pool, other_user_id, "Other folder", None).await;
    let app = build_test_app(&pool).await;

    let boundary = "test-boundary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"folder_id\"\r\n\r\n{foreign_folder_id}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\nContent-Type: text/plain\r\n\r\nHello World\r\n--{boundary}--\r\n"
    );

    let (auth_name, auth_value) = bearer_auth_header(&token);
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

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// 秒传测试 (POST /upload/instant)
// ============================================================================

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_instant_upload_handler_not_found() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "instant_not_found").await;
    let app = build_test_app(&pool).await;

    let body = serde_json::json!({
        "content_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "filename": "test.txt",
        "file_size": 11,
        "mime_type": "text/plain"
    });

    let (auth_name, auth_value) = bearer_auth_header(&token);
    let response = app
        .oneshot(
            axum::http::Request::post("/api/v1/files/upload/instant")
                .header("Content-Type", "application/json")
                .header(auth_name, auth_value)
                .body(AxumBody::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_instant_upload_handler_invalid_hash() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "instant_invalid_hash").await;
    let app = build_test_app(&pool).await;

    let body = serde_json::json!({
        "content_sha256": "invalid-hash",
        "filename": "test.txt",
        "file_size": 11,
        "mime_type": "text/plain"
    });

    let (auth_name, auth_value) = bearer_auth_header(&token);
    let response = app
        .oneshot(
            axum::http::Request::post("/api/v1/files/upload/instant")
                .header("Content-Type", "application/json")
                .header(auth_name, auth_value)
                .body(AxumBody::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_instant_upload_handler_rejects_foreign_folder_id() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (owner_id, token) = login_and_get_token(&pool, "instant_foreign_owner").await;
    let (other_user_id, _, _) = common::create_test_user(&pool, "instant_foreign_other").await;
    let foreign_folder_id =
        common::create_test_folder(&pool, other_user_id, "Other folder", None).await;

    let hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let file_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO files (id, user_id, filename, original_filename, file_path, file_size, mime_type, storage_backend, content_sha256)
         VALUES ($1, $2, 'source.txt', 'source.txt', $3, 11, 'text/plain', 'local', $4)",
    )
    .bind(file_id)
    .bind(owner_id)
    .bind(format!("/test/{owner_id}/{file_id}"))
    .bind(hash)
    .execute(&pool)
    .await
    .unwrap();

    let app = build_test_app(&pool).await;
    let body = serde_json::json!({
        "content_sha256": hash,
        "filename": "copy.txt",
        "file_size": 11,
        "mime_type": "text/plain",
        "folder_id": foreign_folder_id
    });

    let (auth_name, auth_value) = bearer_auth_header(&token);
    let response = app
        .oneshot(
            axum::http::Request::post("/api/v1/files/upload/instant")
                .header("Content-Type", "application/json")
                .header(auth_name, auth_value)
                .body(AxumBody::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// 分块上传测试 (POST /upload/chunked/init)
// ============================================================================

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_chunked_upload_init_handler_happy_path() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "chunked_init_happy").await;
    let app = build_test_app(&pool).await;

    let body = serde_json::json!({
        "filename": "large-file.zip",
        "mime_type": "application/zip",
        "total_size": 1048576
    });

    let (auth_name, auth_value) = bearer_auth_header(&token);
    let response = app
        .oneshot(
            axum::http::Request::post("/api/v1/files/upload/chunked/init")
                .header("Content-Type", "application/json")
                .header(auth_name, auth_value)
                .body(AxumBody::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_chunked_upload_init_handler_zero_size() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "chunked_init_zero").await;
    let app = build_test_app(&pool).await;

    let body = serde_json::json!({
        "filename": "empty.zip",
        "mime_type": "application/zip",
        "total_size": 0
    });

    let (auth_name, auth_value) = bearer_auth_header(&token);
    let response = app
        .oneshot(
            axum::http::Request::post("/api/v1/files/upload/chunked/init")
                .header("Content-Type", "application/json")
                .header(auth_name, auth_value)
                .body(AxumBody::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(result.get("upload_id").is_some());
    assert_eq!(result["total_parts"], 0);
}

// ============================================================================
// 上传分块测试 (PUT /upload/chunked/{id}/chunk)
// ============================================================================

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_chunked_upload_chunk_handler_happy_path() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "chunked_chunk_happy").await;
    let app = build_test_app(&pool).await;
    let (auth_name, auth_value) = bearer_auth_header(&token);

    // 先初始化上传会话
    let init_body = serde_json::json!({
        "filename": "large-file.zip",
        "mime_type": "application/zip",
        "total_size": 15
    });

    let init_response = app
        .clone()
        .oneshot(
            axum::http::Request::post("/api/v1/files/upload/chunked/init")
                .header("Content-Type", "application/json")
                .header(auth_name.clone(), auth_value.clone())
                .body(AxumBody::from(serde_json::to_string(&init_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(init_response.status(), StatusCode::OK);

    // 解析 upload_id
    let init_body_bytes = to_bytes(init_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let init_result: serde_json::Value = serde_json::from_slice(&init_body_bytes).unwrap();
    let upload_id = init_result["upload_id"].as_str().unwrap();

    // 上传分块
    let chunk_data = Bytes::from_static(b"test chunk data");
    let part_sha256 = sha256_hex(&chunk_data);
    let chunk_response = app
        .oneshot(
            axum::http::Request::put(format!(
                "/api/v1/files/upload/chunked/{upload_id}/chunk?part=1"
            ))
            .header("Content-Type", "application/octet-stream")
            .header("X-Part-SHA256", part_sha256)
            .header(auth_name, auth_value)
            .body(AxumBody::from(chunk_data))
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(chunk_response.status(), StatusCode::OK);
}

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_chunked_upload_chunk_handler_rejects_wrong_chunk_size() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "chunked_chunk_wrong_size").await;
    let app = build_test_app(&pool).await;
    let (auth_name, auth_value) = bearer_auth_header(&token);

    let init_body = serde_json::json!({
        "filename": "large-file.zip",
        "mime_type": "application/zip",
        "total_size": 15
    });
    let init_response = app
        .clone()
        .oneshot(
            axum::http::Request::post("/api/v1/files/upload/chunked/init")
                .header("Content-Type", "application/json")
                .header(auth_name.clone(), auth_value.clone())
                .body(AxumBody::from(serde_json::to_string(&init_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(init_response.status(), StatusCode::OK);
    let init_body_bytes = to_bytes(init_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let init_result: serde_json::Value = serde_json::from_slice(&init_body_bytes).unwrap();
    let upload_id = init_result["upload_id"].as_str().unwrap();

    let response = app
        .oneshot(
            axum::http::Request::put(format!(
                "/api/v1/files/upload/chunked/{upload_id}/chunk?part=1"
            ))
            .header("Content-Type", "application/octet-stream")
            .header(auth_name, auth_value)
            .body(AxumBody::from(Bytes::from_static(b"too short")))
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_chunked_upload_chunk_handler_rejects_checksum_mismatch() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "chunked_chunk_bad_sha").await;
    let app = build_test_app(&pool).await;
    let (auth_name, auth_value) = bearer_auth_header(&token);

    let init_body = serde_json::json!({
        "filename": "large-file.zip",
        "mime_type": "application/zip",
        "total_size": 15
    });
    let init_response = app
        .clone()
        .oneshot(
            axum::http::Request::post("/api/v1/files/upload/chunked/init")
                .header("Content-Type", "application/json")
                .header(auth_name.clone(), auth_value.clone())
                .body(AxumBody::from(serde_json::to_string(&init_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(init_response.status(), StatusCode::OK);
    let init_body_bytes = to_bytes(init_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let init_result: serde_json::Value = serde_json::from_slice(&init_body_bytes).unwrap();
    let upload_id = init_result["upload_id"].as_str().unwrap();

    let response = app
        .oneshot(
            axum::http::Request::put(format!(
                "/api/v1/files/upload/chunked/{upload_id}/chunk?part=1"
            ))
            .header("Content-Type", "application/octet-stream")
            .header("X-Part-SHA256", "0".repeat(64))
            .header(auth_name, auth_value)
            .body(AxumBody::from(Bytes::from_static(b"test chunk data")))
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_chunked_upload_chunk_handler_invalid_upload_id() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "chunked_chunk_invalid").await;
    let app = build_test_app(&pool).await;
    let (auth_name, auth_value) = bearer_auth_header(&token);

    let invalid_upload_id = "00000000-0000-0000-0000-000000000000";

    let response = app
        .oneshot(
            axum::http::Request::put(format!(
                "/api/v1/files/upload/chunked/{invalid_upload_id}/chunk?part=1"
            ))
            .header("Content-Type", "application/octet-stream")
            .header(auth_name, auth_value)
            .body(AxumBody::from(Bytes::from_static(b"test chunk data")))
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// 查询分块状态测试 (GET /upload/chunked/{id}/status)
// ============================================================================

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_chunked_upload_status_handler_happy_path() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "chunked_status_happy").await;
    let app = build_test_app(&pool).await;
    let (auth_name, auth_value) = bearer_auth_header(&token);

    // 先初始化上传会话
    let init_body = serde_json::json!({
        "filename": "large-file.zip",
        "mime_type": "application/zip",
        "total_size": 1048576
    });

    let init_response = app
        .clone()
        .oneshot(
            axum::http::Request::post("/api/v1/files/upload/chunked/init")
                .header("Content-Type", "application/json")
                .header(auth_name.clone(), auth_value.clone())
                .body(AxumBody::from(serde_json::to_string(&init_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let init_body_bytes = to_bytes(init_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let init_result: serde_json::Value = serde_json::from_slice(&init_body_bytes).unwrap();
    let upload_id = init_result["upload_id"].as_str().unwrap();

    // 查询状态
    let response = app
        .oneshot(
            axum::http::Request::get(format!("/api/v1/files/upload/chunked/{upload_id}/status"))
                .header(auth_name, auth_value)
                .body(AxumBody::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_chunked_upload_status_handler_not_found() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "chunked_status_not_found").await;
    let app = build_test_app(&pool).await;
    let (auth_name, auth_value) = bearer_auth_header(&token);

    let invalid_upload_id = "00000000-0000-0000-0000-000000000000";

    let response = app
        .oneshot(
            axum::http::Request::get(format!(
                "/api/v1/files/upload/chunked/{invalid_upload_id}/status"
            ))
            .header(auth_name, auth_value)
            .body(AxumBody::empty())
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// 完成分块上传测试 (POST /upload/chunked/{id}/complete)
// ============================================================================

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_chunked_upload_complete_handler_missing_chunks() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "chunked_complete_missing").await;
    let app = build_test_app(&pool).await;
    let (auth_name, auth_value) = bearer_auth_header(&token);

    // 先初始化上传会话
    let init_body = serde_json::json!({
        "filename": "large-file.zip",
        "mime_type": "application/zip",
        "total_size": 104857600  // 100MB to ensure multiple chunks
    });

    let init_response = app
        .clone()
        .oneshot(
            axum::http::Request::post("/api/v1/files/upload/chunked/init")
                .header("Content-Type", "application/json")
                .header(auth_name.clone(), auth_value.clone())
                .body(AxumBody::from(serde_json::to_string(&init_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // 检查初始化响应是否成功
    assert_eq!(
        init_response.status(),
        StatusCode::OK,
        "Failed to initialize upload"
    );

    let init_body_bytes = to_bytes(init_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let init_result: serde_json::Value = serde_json::from_slice(&init_body_bytes).unwrap();
    let upload_id = init_result["upload_id"]
        .as_str()
        .expect("upload_id not found in response");

    // 尝试完成但没有上传任何分块
    let complete_body = serde_json::json!({
        "content_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "folder_id": null
    });

    let response = app
        .oneshot(
            axum::http::Request::post(format!("/api/v1/files/upload/chunked/{upload_id}/complete"))
                .header("Content-Type", "application/json")
                .header(auth_name, auth_value)
                .body(AxumBody::from(
                    serde_json::to_string(&complete_body).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // 实际返回的可能是 BAD_REQUEST 或其他状态码，让我们检查实际行为
    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Expected error status, got: {:?}",
        response.status()
    );
}

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_chunked_upload_complete_handler_rejects_foreign_folder_id() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_owner_id, token) = login_and_get_token(&pool, "chunked_foreign_owner").await;
    let (other_user_id, _, _) = common::create_test_user(&pool, "chunked_foreign_other").await;
    let foreign_folder_id =
        common::create_test_folder(&pool, other_user_id, "Other folder", None).await;
    let app = build_test_app(&pool).await;
    let (auth_name, auth_value) = bearer_auth_header(&token);

    let init_body = serde_json::json!({
        "filename": "chunked.txt",
        "mime_type": "text/plain",
        "total_size": 11
    });
    let init_response = app
        .clone()
        .oneshot(
            axum::http::Request::post("/api/v1/files/upload/chunked/init")
                .header("Content-Type", "application/json")
                .header(auth_name.clone(), auth_value.clone())
                .body(AxumBody::from(serde_json::to_string(&init_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(init_response.status(), StatusCode::OK);
    let init_body_bytes = to_bytes(init_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let init_result: serde_json::Value = serde_json::from_slice(&init_body_bytes).unwrap();
    let upload_id = init_result["upload_id"].as_str().unwrap();

    let chunk_response = app
        .clone()
        .oneshot(
            axum::http::Request::put(format!(
                "/api/v1/files/upload/chunked/{upload_id}/chunk?part=1"
            ))
            .header("Content-Type", "application/octet-stream")
            .header(auth_name.clone(), auth_value.clone())
            .body(AxumBody::from(Bytes::from_static(b"Hello World")))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(chunk_response.status(), StatusCode::OK);

    let complete_body = serde_json::json!({ "folder_id": foreign_folder_id });
    let response = app
        .oneshot(
            axum::http::Request::post(format!("/api/v1/files/upload/chunked/{upload_id}/complete"))
                .header("Content-Type", "application/json")
                .header(auth_name, auth_value)
                .body(AxumBody::from(
                    serde_json::to_string(&complete_body).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// 取消分块上传测试 (DELETE /upload/chunked/{id}/abort)
// ============================================================================

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_chunked_upload_abort_handler_happy_path() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "chunked_abort_happy").await;
    let app = build_test_app(&pool).await;
    let (auth_name, auth_value) = bearer_auth_header(&token);

    // 先初始化上传会话
    let init_body = serde_json::json!({
        "filename": "large-file.zip",
        "mime_type": "application/zip",
        "total_size": 1048576
    });

    let init_response = app
        .clone()
        .oneshot(
            axum::http::Request::post("/api/v1/files/upload/chunked/init")
                .header("Content-Type", "application/json")
                .header(auth_name.clone(), auth_value.clone())
                .body(AxumBody::from(serde_json::to_string(&init_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // 检查初始化响应是否成功
    assert_eq!(
        init_response.status(),
        StatusCode::OK,
        "Failed to initialize upload"
    );

    let init_body_bytes = to_bytes(init_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let init_result: serde_json::Value = serde_json::from_slice(&init_body_bytes).unwrap();
    let upload_id = init_result["upload_id"]
        .as_str()
        .expect("upload_id not found in response");

    // 取消上传
    let response = app
        .oneshot(
            axum::http::Request::delete(format!("/api/v1/files/upload/chunked/{upload_id}/abort"))
                .header(auth_name, auth_value)
                .body(AxumBody::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[serial(upload_handler_db)]
async fn test_chunked_upload_abort_handler_not_found() {
    init_test_env();
    let pool = create_test_pool().await;
    cleanup_test_data(&pool).await;

    let (_user_id, token) = login_and_get_token(&pool, "chunked_abort_not_found").await;
    let app = build_test_app(&pool).await;
    let (auth_name, auth_value) = bearer_auth_header(&token);

    let invalid_upload_id = "00000000-0000-0000-0000-000000000000";

    let response = app
        .oneshot(
            axum::http::Request::delete(format!(
                "/api/v1/files/upload/chunked/{invalid_upload_id}/abort"
            ))
            .header(auth_name, auth_value)
            .body(AxumBody::empty())
            .unwrap(),
        )
        .await
        .unwrap();

    // 即使会话不存在，也会返回 200 OK，因为删除操作是幂等的
    assert_eq!(response.status(), StatusCode::OK);
}
