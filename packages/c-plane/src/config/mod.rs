use std::env;

use crate::errors::{AppError, ConfigError};

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub kratos_api_key: String,
}

pub fn load_config() -> Result<Config, AppError> {
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL").map_err(|_| ConfigError::MissingDatabaseUrl)?;
    if database_url.trim().is_empty() {
        return Err(AppError::Config(ConfigError::MissingDatabaseUrl));
    }

    let kratos_api_key = env::var("KRATOS_API_KEY").map_err(|_| ConfigError::MissingKratosApiKey)?;
    if kratos_api_key.trim().is_empty() {
        return Err(AppError::Config(ConfigError::MissingKratosApiKey));
    }

    let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

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
        kratos_api_key
    })
}
