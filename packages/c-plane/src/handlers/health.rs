use actix_web::{HttpResponse, Result, get, web};
use serde::{Serialize};

use crate::errors::{AppError};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_health_handler);
}

#[derive(Serialize)]
struct GetHealthResponse {
    status: String,
    message: String,
}

#[get("/health")]
async fn get_health_handler() -> Result<HttpResponse, AppError> {
    Ok(HttpResponse::Ok().json(GetHealthResponse {
        status: "Running".to_string(),
        message: "Hello from Rust".to_string(),
    }))
}
