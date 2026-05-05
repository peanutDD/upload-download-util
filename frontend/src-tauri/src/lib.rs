#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let backend_base_url = std::env::var("UPLOAD_DOWNLOAD_UTIL_API_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());

    let client = reqwest::Client::new();
    let repo = std::sync::Arc::new(repositories::backend_repository::BackendRepository::new(
        client,
        backend_base_url,
    ));
    let backend_service = services::backend_service::BackendService::new(repo);

    tauri::Builder::default()
        .manage(AppState { backend_service })
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::health::backend_health])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

mod commands;
mod models;
mod repositories;
mod services;

#[derive(Clone)]
pub struct AppState {
    pub(crate) backend_service: services::backend_service::BackendService,
}
