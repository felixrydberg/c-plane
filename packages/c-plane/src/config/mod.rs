use std::env;

use crate::errors::{AppError, ConfigError};

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
}

pub fn load_config() -> Result<Config, AppError> {
    dotenvy::dotenv().ok();

    // DATABASE_URL is critical - application cannot function without it
    let database_url = env::var("DATABASE_URL").map_err(|_| ConfigError::MissingDatabaseUrl)?;

    if database_url.trim().is_empty() {
        return Err(AppError::Config(ConfigError::MissingDatabaseUrl));
    }

    let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "localhost".to_string());

    let server_port = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .map_err(|_| {
            ConfigError::InvalidServerPort(
                env::var("SERVER_PORT").unwrap_or_else(|_| "invalid".to_string()),
            )
        })?;

    Ok(Config {
        database_url,
        server_host,
        server_port,
    })
}
