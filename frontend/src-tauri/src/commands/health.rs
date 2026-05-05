use tauri::State;

use crate::models::error::AppError;
use crate::models::HealthResponse;
use crate::AppState;

#[tauri::command]
pub async fn backend_health(state: State<'_, AppState>) -> Result<HealthResponse, AppError> {
    state.backend_service.health().await
}
