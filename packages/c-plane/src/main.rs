mod config;
mod errors;
mod routes;
mod models;
mod middleware;
mod state;
mod handlers;
mod utils;

use crate::errors::AppError;
use crate::state::create_app_state;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    create_app_state().await?;
    let config = config::load_config()?;

    let app = routes::create_routes();
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.server_host, config.server_port))
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;
    axum::serve(listener, app)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(())
}
