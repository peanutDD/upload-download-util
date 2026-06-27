use std::sync::Arc;
use std::time::Duration;

use axum::routing::get;
use axum::Router;
use metrics::gauge;
use serde_json::json;
use tokio::sync::Semaphore;

use file_storage_backend::config::Config;
use file_storage_backend::database::pool::create_pool;
use file_storage_backend::middleware::metrics::init_metrics;
use file_storage_backend::services::file::create_storage;
use file_storage_backend::services::task_queue::{
    run_fulltext_index_worker, run_fulltext_remove_worker, run_gif_preview_worker, run_hls_worker,
    run_trash_cleanup_worker,
};
use file_storage_backend::AppState;

use file_storage_backend::tracing::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let metrics_renderer = init_metrics()
        .map_err(|e| anyhow::anyhow!("Failed to install Prometheus recorder: {}", e))?;

    let config = Arc::new(Config::from_env()?);
    let pool = create_pool(&config.database.url).await?;
    let read_pool = match config.database.read_replica_url.as_deref() {
        Some(url) => create_pool(url).await?,
        None => pool.clone(),
    };
    let storage = create_storage(config.clone()).await?;
    let state = AppState::new(config.clone(), pool, read_pool, storage, None);

    let concurrency: usize = std::env::var("WORKER_CONCURRENCY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2);

    let transcode_semaphore = Arc::new(Semaphore::new(config.tasks.transcode_max_concurrent));

    // ---------- GIF preview worker ----------
    let gif_preview_semaphore = config
        .tasks
        .task_type_concurrency
        .get("gif_preview")
        .copied()
        .map(|n| Arc::new(Semaphore::new(n)));

    for _ in 0..concurrency {
        let state_for_worker = state.clone();
        let transcode_semaphore = transcode_semaphore.clone();
        let gif_preview_semaphore = gif_preview_semaphore.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = run_gif_preview_worker(
                    &state_for_worker,
                    transcode_semaphore.clone(),
                    gif_preview_semaphore.clone(),
                )
                .await
                {
                    tracing::error!("gif_preview worker iteration failed: {}", e);
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
    }

    // ---------- HLS preview worker ----------
    // 与 gif_preview 完全对称：并发配额 + 指数退避重试 + Prometheus 指标
    let hls_preview_semaphore = config
        .tasks
        .task_type_concurrency
        .get("hls_preview")
        .copied()
        .map(|n| Arc::new(Semaphore::new(n)));

    for _ in 0..concurrency {
        let state_for_hls = state.clone();
        let transcode_semaphore = transcode_semaphore.clone();
        let hls_preview_semaphore = hls_preview_semaphore.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = run_hls_worker(
                    &state_for_hls,
                    transcode_semaphore.clone(),
                    hls_preview_semaphore.clone(),
                )
                .await
                {
                    tracing::error!("hls_preview worker iteration failed: {}", e);
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
    }

    // ---------- Trash cleanup scheduler + worker ----------
    {
        let state_for_scheduler = state.clone();
        tokio::spawn(async move {
            loop {
                let cleanup_slot = chrono::Utc::now().format("%Y-%m-%dT%H:%M").to_string();
                let dedupe_key = format!("trash_cleanup:{}", cleanup_slot);
                let payload = json!({
                    "retention_days": 30,
                    "batch_limit": 500,
                });
                if let Err(e) = state_for_scheduler
                    .task_queue
                    .enqueue_task("trash_cleanup", payload, Some(&dedupe_key))
                    .await
                {
                    tracing::warn!(error = %e, "failed to enqueue trash cleanup");
                }
                tokio::time::sleep(Duration::from_secs(60 * 60)).await;
            }
        });
    }

    {
        let state_for_trash = state.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = run_trash_cleanup_worker(&state_for_trash).await {
                    tracing::error!("trash_cleanup worker iteration failed: {}", e);
                }
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });
    }

    for _ in 0..concurrency {
        let state_for_fulltext = state.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = run_fulltext_index_worker(&state_for_fulltext).await {
                    tracing::error!("search_index_file worker iteration failed: {}", e);
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
    }

    for _ in 0..concurrency {
        let state_for_fulltext = state.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = run_fulltext_remove_worker(&state_for_fulltext).await {
                    tracing::error!("search_remove_file worker iteration failed: {}", e);
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
    }

    // ---------- Maintenance：requeue stuck tasks ----------
    {
        let state_for_maintenance = state.clone();
        tokio::spawn(async move {
            loop {
                for task_type in [
                    "gif_preview",
                    "hls_preview",
                    "trash_cleanup",
                    "search_index_file",
                    "search_remove_file",
                ] {
                    if let Ok(n) = state_for_maintenance
                        .task_queue
                        .requeue_stuck_tasks(task_type, 200)
                        .await
                    {
                        if n > 0 {
                            tracing::warn!(requeued = n, task_type, "requeued stuck tasks");
                        }
                    }
                }
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });
    }

    // ---------- Metrics：queue depth ----------
    {
        let state_for_metrics = state.clone();
        tokio::spawn(async move {
            loop {
                for task_type in [
                    "gif_preview",
                    "hls_preview",
                    "trash_cleanup",
                    "search_index_file",
                    "search_remove_file",
                ] {
                    if let Ok(depth) = state_for_metrics
                        .task_queue
                        .get_queue_depth(task_type)
                        .await
                    {
                        gauge!("background_tasks_pending_total", "task_type" => task_type)
                            .set(depth.pending_total as f64);
                        gauge!("background_tasks_pending_ready", "task_type" => task_type)
                            .set(depth.pending_ready as f64);
                        gauge!("background_tasks_running", "task_type" => task_type)
                            .set(depth.running as f64);
                        gauge!("background_tasks_failed", "task_type" => task_type)
                            .set(depth.failed as f64);
                    }
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }

    let metrics_handler = {
        let renderer = metrics_renderer.clone();
        move || {
            let r = renderer.clone();
            async move { r() }
        }
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/metrics", get(metrics_handler));

    let host = std::env::var("WORKER_HOST").ok();
    let port = std::env::var("WORKER_PORT").ok();
    let addr = worker_bind_addr(host.as_deref(), port.as_deref());
    tracing::info!("Worker listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn worker_bind_addr(host: Option<&str>, port: Option<&str>) -> String {
    let host = host.unwrap_or("127.0.0.1");
    let port: u16 = port.and_then(|value| value.parse().ok()).unwrap_or(3001);
    format!("{host}:{port}")
}

#[cfg(test)]
mod tests {
    use super::worker_bind_addr;

    #[test]
    fn worker_metrics_bind_addr_defaults_to_loopback() {
        assert_eq!(worker_bind_addr(None, None), "127.0.0.1:3001");
    }

    #[test]
    fn worker_metrics_bind_addr_allows_explicit_host_and_port() {
        assert_eq!(
            worker_bind_addr(Some("0.0.0.0"), Some("4001")),
            "0.0.0.0:4001"
        );
    }
}
