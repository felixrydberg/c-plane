use crate::errors::AppError;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone, Deserialize)]
pub struct S3ProviderCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

#[derive(Clone)]
pub struct S3ProviderClient {
    http: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl S3ProviderClient {
    pub fn new(base_url: String, service_token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            service_token,
        }
    }

    pub async fn credentials(&self, provider_id: Uuid) -> Result<S3ProviderCredentials, AppError> {
        let response = self
            .http
            .get(format!(
                "{}/internal/s3-providers/{provider_id}/credentials",
                self.base_url
            ))
            .header("x-cplane-token", &self.service_token)
            .send()
            .await
            .map_err(|error| {
                AppError::Internal(format!("Control-plane request failed: {error}"))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::NotFound("S3 provider not found".into()));
        }
        if !response.status().is_success() {
            return Err(AppError::Internal(format!(
                "Control-plane returned {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|error| AppError::Internal(format!("Invalid control-plane response: {error}")))
    }
}
