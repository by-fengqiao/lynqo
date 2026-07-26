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

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                "INVALID_TOKEN",
                "Invalid or missing token",
            ),
            AppError::DeviceNotApproved => (
                StatusCode::FORBIDDEN,
                "DEVICE_NOT_APPROVED",
                "Device not approved",
            ),
            AppError::TransferNotFound => (
                StatusCode::NOT_FOUND,
                "TRANSFER_NOT_FOUND",
                "Transfer not found",
            ),
            AppError::DeviceNotFound => (
                StatusCode::NOT_FOUND,
                "DEVICE_NOT_FOUND",
                "Device not found",
            ),
            AppError::InvalidFilename => (
                StatusCode::BAD_REQUEST,
                "INVALID_FILENAME",
                "Invalid filename",
            ),
            AppError::TransferCancelled => (
                StatusCode::CONFLICT,
                "TRANSFER_CANCELLED",
                "Transfer already cancelled",
            ),
            AppError::TransferCompleted => (
                StatusCode::CONFLICT,
                "TRANSFER_COMPLETED",
                "Transfer already completed",
            ),
            AppError::TransferPaused => (
                StatusCode::CONFLICT,
                "TRANSFER_PAUSED",
                "Transfer is paused",
            ),
            AppError::ChunkOutOfRange => (
                StatusCode::BAD_REQUEST,
                "CHUNK_OUT_OF_RANGE",
                "Chunk index out of range",
            ),
            AppError::InvalidRequest(message) => {
                (StatusCode::BAD_REQUEST, "INVALID_REQUEST", message.as_str())
            }
            AppError::Io(_) | AppError::Database(_) | AppError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "Internal server error",
            ),
        };

        tracing::error!("Request error: {}", self);

        let body = Json(json!({
            "error": {
                "code": code,
                "message": message,
            },
        }));

        (status, body).into_response()
    }
}
