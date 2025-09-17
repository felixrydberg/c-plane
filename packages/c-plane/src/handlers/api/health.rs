use actix_web::{HttpResponse, Result, get, post, web};
use serde::{Deserialize, Serialize};

use crate::errors::{AppError};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/health")
            .service(get_health_handler)
    );
}

#[derive(Serialize, Deserialize)]
struct GetHealthRequest {}

#[derive(Serialize, Deserialize)]
struct GetHealthResponse {
  status: String,
  message: String,
}

#[get("/")]
async fn get_health_handler(
    request: web::Json<GetHealthRequest>,
) -> Result<HttpResponse, AppError> {
  Ok(HttpResponse::Ok().json(GetHealthResponse {
    status: "Running".to_string(),
    message: "Hello from Rust".to_string(),
  }))
}
