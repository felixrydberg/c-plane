pub mod config;
pub mod database;
pub mod external;
pub mod organisation;
pub mod project;
pub mod user;

pub use config::ConfigError;
pub use database::DatabaseError;
pub use external::ExternalError;
pub use organisation::OrganisationError;
pub use project::ProjectError;
pub use user::UserError;

use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Project(ProjectError),
    User(UserError),
    Organisation(OrganisationError),

    Database(DatabaseError),
    External(ExternalError),

    Config(ConfigError),

    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),

    Internal(String),
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        match err {
            sea_orm::DbErr::RecordNotFound(_) => AppError::NotFound("Record not found".to_string()),
            sea_orm::DbErr::ConnectionAcquire(_) => {
                AppError::Database(DatabaseError::ConnectionFailed(err.to_string()))
            }
            _ => AppError::Database(DatabaseError::QueryFailed(err.to_string())),
        }
    }
}

impl From<ProjectError> for AppError {
    fn from(err: ProjectError) -> Self {
        AppError::Project(err)
    }
}

impl From<ConfigError> for AppError {
    fn from(err: ConfigError) -> Self {
        AppError::Config(err)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Project(err) => write!(f, "Project error: {}", err),
            AppError::User(err) => write!(f, "User error: {}", err),
            AppError::Organisation(err) => write!(f, "Organisation error: {}", err),
            AppError::Database(err) => write!(f, "Database error: {}", err),
            AppError::External(err) => write!(f, "External error: {}", err),
            AppError::Config(err) => write!(f, "Config error: {}", err),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::Project(_) | AppError::User(_) | AppError::Organisation(_) => {
                HttpResponse::BadRequest().json(ErrorResponse {
                    error: "validation_error".to_string(),
                    message: self.to_string(),
                    details: None,
                })
            }
            AppError::Database(_) | AppError::External(_) | AppError::Internal(_) => {
                HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "internal_error".to_string(),
                    message: "An internal error occurred".to_string(),
                    details: None,
                })
            }
            AppError::Config(_) => HttpResponse::InternalServerError().json(ErrorResponse {
                error: "configuration_error".to_string(),
                message: "Service configuration error".to_string(),
                details: None,
            }),
            AppError::Unauthorized(msg) => HttpResponse::Unauthorized().json(ErrorResponse {
                error: "unauthorized".to_string(),
                message: msg.clone(),
                details: None,
            }),
            AppError::Forbidden(msg) => HttpResponse::Forbidden().json(ErrorResponse {
                error: "forbidden".to_string(),
                message: msg.clone(),
                details: None,
            }),
            AppError::NotFound(msg) => HttpResponse::NotFound().json(ErrorResponse {
                error: "not_found".to_string(),
                message: msg.clone(),
                details: None,
            }),
            AppError::Conflict(msg) => HttpResponse::Conflict().json(ErrorResponse {
                error: "conflict".to_string(),
                message: msg.clone(),
                details: None,
            }),
        }
    }
}
