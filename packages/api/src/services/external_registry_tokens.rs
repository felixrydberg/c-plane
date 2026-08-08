use serde::Serialize;
use std::time::Duration;
use uuid::Uuid;

use crate::errors::AppError;

#[derive(Serialize)]
struct Token<'a> {
    token: &'a str,
}

#[derive(Clone)]
pub struct ExternalRegistryTokenClient {
    http: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl ExternalRegistryTokenClient {
    pub fn new(base_url: String, service_token: String) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("failed to build external registry token client"),
            base_url: base_url.trim_end_matches('/').to_string(),
            service_token,
        }
    }

    pub async fn store(
        &self,
        organization_id: Uuid,
        registry_id: Uuid,
        token: &str,
    ) -> Result<(), AppError> {
        self.request(
            reqwest::Method::PUT,
            organization_id,
            registry_id,
            Some(Token { token }),
        )
        .await
    }

    pub async fn delete(&self, organization_id: Uuid, registry_id: Uuid) -> Result<(), AppError> {
        self.request(reqwest::Method::DELETE, organization_id, registry_id, None)
            .await
    }

    async fn request(
        &self,
        method: reqwest::Method,
        organization_id: Uuid,
        registry_id: Uuid,
        body: Option<Token<'_>>,
    ) -> Result<(), AppError> {
        let mut request = self
            .http
            .request(
                method,
                format!(
                    "{}/internal/organizations/{organization_id}/external-registries/{registry_id}/secret",
                    self.base_url
                ),
            )
            .header("x-cplane-token", &self.service_token);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.map_err(|error| {
            AppError::Internal(format!("Control-plane request failed: {error}"))
        })?;
        if !response.status().is_success() {
            return Err(AppError::Internal(format!(
                "Control-plane returned {}",
                response.status()
            )));
        }
        Ok(())
    }
}
