mod config;
mod errors;
mod handlers;
mod middleware;
mod models;
mod services;
mod state;
mod utils;

use crate::state::create_app_state;
use crate::utils::logger::CustomLogger;
use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if let Err(e) = create_app_state().await {
        eprintln!("Failed to create app state: {}", e);
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("App state creation failed: {}", e),
        ));
    }

    let config = config::load_config()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    HttpServer::new(move || {
        App::new()
            .wrap(CustomLogger)
            .configure(handlers::api::config)
    })
    .bind(format!("{}:{}", config.server_host, config.server_port))?
    .run()
    .await
}
