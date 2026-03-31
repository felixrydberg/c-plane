use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
};

use crate::handlers::{
    health::health_check,
    
};
use crate::middleware::auth::auth_middleware;

pub fn create_routes() -> Router {
    // let protected = Router::new()
    //     .route(
    //         "/storage/verification/{verification}/training",
    //         post(ingest_verification),
    //     )
    //     .route(
    //         "/storage/verification/{verification}/manual",
    //         post(ingest_manual_review),
    //     )
    //     .route(
    //         "/storage/verification/{verification}/manual/assets/{asset}",
    //         get(get_manual_review_asset),
    //     )
    //     .layer(middleware::from_fn(auth_middleware));

    Router::new()
        .route("/health", get(health_check))
        // .merge(protected)
        .layer(DefaultBodyLimit::max(1 * 1024 * 1024 * 1024)) // 1GB limit
}
