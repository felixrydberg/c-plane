use crate::errors::AppError;
use crate::state::create_app_state;
use tower_http::trace::TraceLayer;
use tracing::Level;
use tracing_subscriber::filter::EnvFilter;
use utoipa::OpenApi;

mod config;
mod errors;
mod handlers;
mod middleware;
mod models;
mod openapi;
mod routes;
mod services;
mod state;
mod utils;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(Level::WARN.into())
                .from_env_lossy()
                .add_directive("c_plane=info".parse().unwrap()),
        )
        .with_target(false)
        .init();

    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--openapi") {
        let output_path = args
            .next()
            .ok_or_else(|| AppError::Internal("--openapi requires an output path".into()))?;
        let document = serde_json::to_vec_pretty(&openapi::ApiDoc::openapi())
            .map_err(|err| AppError::Internal(err.to_string()))?;
        std::fs::write(output_path, document).map_err(|err| AppError::Internal(err.to_string()))?;
        return Ok(());
    }

    let config = create_app_state().await?.config;

    let app = routes::create_routes().layer(TraceLayer::new_for_http());

    let addr = format!("{}:{}", config.server_host, config.server_port);
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    axum::serve(listener, app)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(())
}
