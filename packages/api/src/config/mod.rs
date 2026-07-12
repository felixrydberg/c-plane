use std::env;

use crate::errors::{AppError, ConfigError};

#[derive(Clone)]
pub struct Config {
    pub identity_database_url: String,
    pub tenant_database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub better_auth_session_url: String,
    pub control_plane_url: Option<String>,
    pub control_plane_service_token: Option<String>,
}

pub fn load_config() -> Result<Config, AppError> {
    dotenvy::dotenv().ok();

    let identity_database_url =
        env::var("IDENTITY_DATABASE_URL").map_err(|_| ConfigError::MissingIdentityDatabaseUrl)?;
    if identity_database_url.trim().is_empty() {
        return Err(AppError::Config(ConfigError::MissingIdentityDatabaseUrl));
    }

    let tenant_database_url =
        env::var("TENANT_DATABASE_URL").map_err(|_| ConfigError::MissingTenantDatabaseUrl)?;
    if tenant_database_url.trim().is_empty() {
        return Err(AppError::Config(ConfigError::MissingTenantDatabaseUrl));
    }

    let better_auth_session_url = env::var("BETTER_AUTH_SESSION_URL")
        .unwrap_or_else(|_| "http://ui:3000/api/auth/get-session".to_string());
    let control_plane_url = env::var("CONTROL_PLANE_URL")
        .ok()
        .filter(|v| !v.trim().is_empty());
    let control_plane_service_token = env::var("CPLANE_SERVICE_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty());

    let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

    let server_port = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .map_err(|_| {
            ConfigError::InvalidServerPort(
                env::var("SERVER_PORT").unwrap_or_else(|_| "invalid".to_string()),
            )
        })?;

    Ok(Config {
        identity_database_url,
        tenant_database_url,
        server_host,
        server_port,
        better_auth_session_url,
        control_plane_url,
        control_plane_service_token,
    })
}
