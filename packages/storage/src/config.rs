use std::{env, net::SocketAddr};

use thiserror::Error;

#[derive(Clone)]
pub struct Config {
    pub listen: SocketAddr,
    pub control_plane_url: String,
    pub internal_token: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing environment variable {0}")]
    Missing(&'static str),
    #[error("invalid environment variable {0}")]
    Invalid(&'static str),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let required = |name: &'static str| {
            env::var(name)
                .ok()
                .filter(|value| !value.trim().is_empty())
                .ok_or(ConfigError::Missing(name))
        };
        let listen = env::var("STORAGE_LISTEN")
            .unwrap_or_else(|_| "0.0.0.0:8081".into())
            .parse()
            .map_err(|_| ConfigError::Invalid("STORAGE_LISTEN"))?;
        Ok(Self {
            listen,
            control_plane_url: required("CONTROL_PLANE_URL")?
                .trim_end_matches('/')
                .to_string(),
            internal_token: required("CPLANE_SERVICE_TOKEN")?,
        })
    }
}
