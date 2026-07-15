use axum::{http::StatusCode, response::Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub message: String,
}

pub async fn check_health() -> (StatusCode, Json<HealthResponse>) {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "healthy".to_string(),
            message: "Dejavu Similarity API is running".to_string(),
        }),
    )
}
