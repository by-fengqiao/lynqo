use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Invalid or missing token")]
    InvalidToken,

    #[error("Device not approved")]
    DeviceNotApproved,

    #[error("Transfer not found")]
    TransferNotFound,

    #[error("Device not found")]
    DeviceNotFound,

    #[error("Invalid filename")]
    InvalidFilename,

    #[error("Transfer already cancelled")]
    TransferCancelled,

    #[error("Transfer already completed")]
    TransferCompleted,

    #[error("Transfer is paused")]
    TransferPaused,

    #[error("Chunk index out of range")]
    ChunkOutOfRange,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid or missing token"),
            AppError::DeviceNotApproved => (StatusCode::FORBIDDEN, "Device not approved"),
            AppError::TransferNotFound => (StatusCode::NOT_FOUND, "Transfer not found"),
            AppError::DeviceNotFound => (StatusCode::NOT_FOUND, "Device not found"),
            AppError::InvalidFilename => (StatusCode::BAD_REQUEST, "Invalid filename"),
            AppError::TransferCancelled => (StatusCode::CONFLICT, "Transfer already cancelled"),
            AppError::TransferCompleted => (StatusCode::CONFLICT, "Transfer already completed"),
            AppError::TransferPaused => (StatusCode::CONFLICT, "Transfer is paused"),
            AppError::ChunkOutOfRange => (StatusCode::BAD_REQUEST, "Chunk index out of range"),
            AppError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
        };

        tracing::error!("Request error: {}", self);

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}
