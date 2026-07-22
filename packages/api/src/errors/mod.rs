use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use std::fmt;
use utoipa::ToSchema;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    ServiceUnavailable(String),

    Internal(String),
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        match err {
            sea_orm::DbErr::RecordNotFound(_) => AppError::NotFound("Record not found".to_string()),
            _ => AppError::Internal(err.to_string()),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::ServiceUnavailable(msg) => write!(f, "Service unavailable: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let error_msg = self.to_string();
        tracing::error!("Request error: {}", error_msg);

        let (status, body) = match self {
            AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorResponse {
                    error: "validation_error".to_string(),
                    message: msg,
                    details: None,
                },
            ),
            AppError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse {
                    error: "internal_error".to_string(),
                    message: "An internal error occurred".to_string(),
                    details: None,
                },
            ),
            AppError::Unauthorized(msg) => (
                StatusCode::UNAUTHORIZED,
                ErrorResponse {
                    error: "unauthorized".to_string(),
                    message: msg,
                    details: None,
                },
            ),
            AppError::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                ErrorResponse {
                    error: "forbidden".to_string(),
                    message: msg,
                    details: None,
                },
            ),
            AppError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                ErrorResponse {
                    error: "not_found".to_string(),
                    message: msg,
                    details: None,
                },
            ),
            AppError::Conflict(msg) => (
                StatusCode::CONFLICT,
                ErrorResponse {
                    error: "conflict".to_string(),
                    message: msg,
                    details: None,
                },
            ),
            AppError::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                ErrorResponse {
                    error: "service_unavailable".to_string(),
                    message: msg,
                    details: None,
                },
            ),
        };

        (status, Json(body)).into_response()
    }
}
