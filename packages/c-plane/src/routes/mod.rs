use axum::{
    Router,
    routing::any,
    routing::get,
};

use crate::handlers::{
    health::health_check,
};

pub fn create_routes() -> Router {

    Router::new()
        .route("/health", get(health_check))
}
