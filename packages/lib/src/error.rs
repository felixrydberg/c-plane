use std::fmt;

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

impl From<sea_orm::DbErr> for AppError {
    fn from(error: sea_orm::DbErr) -> Self {
        match error {
            sea_orm::DbErr::RecordNotFound(_) => AppError::NotFound("Record not found".into()),
            error => AppError::Internal(error.to_string()),
        }
    }
}

impl From<crate::secrets::SecretError> for AppError {
    fn from(error: crate::secrets::SecretError) -> Self {
        AppError::Internal(error.to_string())
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::BadRequest(message) => write!(formatter, "Bad request: {message}"),
            AppError::Unauthorized(message) => write!(formatter, "Unauthorized: {message}"),
            AppError::Forbidden(message) => write!(formatter, "Forbidden: {message}"),
            AppError::NotFound(message) => write!(formatter, "Not found: {message}"),
            AppError::Conflict(message) => write!(formatter, "Conflict: {message}"),
            AppError::ServiceUnavailable(message) => {
                write!(formatter, "Service unavailable: {message}")
            }
            AppError::Internal(message) => write!(formatter, "Internal error: {message}"),
        }
    }
}

impl std::error::Error for AppError {}
