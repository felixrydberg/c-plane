use std::fmt;

#[derive(Debug)]
pub enum ConfigError {
    MissingIdentityDatabaseUrl,
    MissingTenantDatabaseUrl,
    MissingBetterAuthSessionUrl,
    InvalidServerPort(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingIdentityDatabaseUrl => {
                write!(
                    f,
                    "IDENTITY_DATABASE_URL environment variable is required and cannot be empty"
                )
            }
            ConfigError::MissingTenantDatabaseUrl => {
                write!(
                    f,
                    "TENANT_DATABASE_URL environment variable is required and cannot be empty"
                )
            }
            ConfigError::MissingBetterAuthSessionUrl => {
                write!(
                    f,
                    "BETTER_AUTH_SESSION_URL environment variable is required and cannot be empty"
                )
            }
            ConfigError::InvalidServerPort(port) => {
                write!(f, "SERVER_PORT '{}' is not a valid port number", port)
            }
        }
    }
}

impl std::error::Error for ConfigError {}
