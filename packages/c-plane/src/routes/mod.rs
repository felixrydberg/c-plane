use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::get,
};

use crate::handlers::{
    health::health_check,
};

pub fn create_routes() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .layer(DefaultBodyLimit::max(1 * 1024 * 1024 * 1024)) // 1GB limit
}
