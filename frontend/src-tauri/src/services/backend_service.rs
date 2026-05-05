use std::sync::Arc;

use crate::models::error::AppError;
use crate::models::HealthResponse;
use crate::repositories::backend_repository::BackendRepository;

#[derive(Clone)]
pub struct BackendService {
    repo: Arc<BackendRepository>,
}

impl BackendService {
    pub fn new(repo: Arc<BackendRepository>) -> Self {
        Self { repo }
    }

    pub async fn health(&self) -> Result<HealthResponse, AppError> {
        self.repo.health().await
    }
}
