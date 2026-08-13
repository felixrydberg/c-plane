use std::env;

use crate::errors::AppError;

#[derive(Clone)]
pub struct Config {
    pub identity_database_url: String,
    pub tenant_database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub redis_url: String,
    pub service_token: String,
    pub storage_endpoint_url: String,
    pub registry_token_ttl_seconds: u64,
}

pub fn load_config() -> Result<Config, AppError> {
    dotenvy::dotenv().ok();

    let identity_database_url = env::var("IDENTITY_DATABASE_URL")
        .map_err(|_| AppError::Internal("IDENTITY_DATABASE_URL is required".into()))?;
    if identity_database_url.trim().is_empty() {
        return Err(AppError::Internal(
            "IDENTITY_DATABASE_URL is required".into(),
        ));
    }

    let tenant_database_url = env::var("TENANT_DATABASE_URL")
        .map_err(|_| AppError::Internal("TENANT_DATABASE_URL is required".into()))?;
    if tenant_database_url.trim().is_empty() {
        return Err(AppError::Internal("TENANT_DATABASE_URL is required".into()));
    }

    let redis_url =
        env::var("REDIS_URL").map_err(|_| AppError::Internal("REDIS_URL is required".into()))?;
    let service_token = env::var("CPLANE_SERVICE_TOKEN")
        .map_err(|_| AppError::Internal("CPLANE_SERVICE_TOKEN is required".into()))?;
    let storage_endpoint_url =
        env::var("STORAGE_ENDPOINT_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    let registry_token_ttl_seconds = env::var("REGISTRY_TOKEN_TTL_SECONDS")
        .unwrap_or_else(|_| "60".to_string())
        .parse()
        .map_err(|_| AppError::Internal("REGISTRY_TOKEN_TTL_SECONDS must be a number".into()))?;
    if registry_token_ttl_seconds < 60 {
        return Err(AppError::Internal(
            "REGISTRY_TOKEN_TTL_SECONDS must be at least 60".into(),
        ));
    }

    let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

    let server_port = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .map_err(|_| AppError::Internal("SERVER_PORT must be a valid port".into()))?;

    Ok(Config {
        identity_database_url,
        tenant_database_url,
        server_host,
        server_port,
        redis_url,
        service_token,
        storage_endpoint_url,
        registry_token_ttl_seconds,
    })
}
