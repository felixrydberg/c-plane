mod config;
mod errors;
mod routes;
mod models;
mod services;
mod middleware;
mod state;
mod handlers;
mod utils;

use crate::errors::AppError;
use crate::state::create_app_state;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    create_app_state().await?;
    let config = config::load_config()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let app = routes::create_routes();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(())
}
