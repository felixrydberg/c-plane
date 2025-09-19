use std::fmt;

#[derive(Debug)]
pub enum ConfigError {
    MissingDatabaseUrl,
    MissingKratosApiKey,
    InvalidServerPort(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingDatabaseUrl => {
                write!(
                    f,
                    "DATABASE_URL environment variable is required and cannot be empty"
                )
            }
            ConfigError::MissingKratosApiKey => {
                write!(
                    f,
                    "KRATOS_API_KEY environment variable is required and cannot be empty"
                )
            }
            ConfigError::InvalidServerPort(port) => {
                write!(f, "SERVER_PORT '{}' is not a valid port number", port)
            }
        }
    }
}

impl std::error::Error for ConfigError {}
