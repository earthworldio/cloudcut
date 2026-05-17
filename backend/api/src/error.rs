use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

/* โครงสร้าง Error Response ตาม Blueprint */
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub status_code: u16,
    pub error: String,
    pub message: String,
    pub request_id: String,
}

/* AppError enum ตามที่ระบุใน blueprint */
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Database error")]
    Database(#[from] sqlx::Error),

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}

/* Implement IntoResponse เพื่อแปลง AppError เป็น JSON response อัตโนมัติ */
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_msg) = match self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };

        /* สร้าง Request ID (ใช้ UUID v7 ตามโจทย์) */
        let request_id = Uuid::now_v7().to_string();

        let body = Json(ErrorResponse {
            status_code: status.as_u16(),
            error: status.canonical_reason().unwrap_or("Unknown").to_string(),
            message: error_msg,
            request_id,
        });

        (status, body).into_response()
    }
}
