use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Bad request: {0}")]
    #[allow(dead_code)]
    BadRequest(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message, error_type) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg, "Not Found"),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg, "Conflict"),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg, "Bad Request"),
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg, "Validation Error"),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg, "Unauthorized"),
            AppError::InternalError(msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    msg,
                    "Internal Server Error",
                )
            }
        };

        let body = Json(json!({
            "message": message,
            "error": error_type,
            "statusCode": status.as_u16(),
        }));

        (status, body).into_response()
    }
}
