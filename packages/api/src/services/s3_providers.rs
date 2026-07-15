use crate::errors::AppError;
use aws_sdk_s3::{
    config::{BehaviorVersion, Credentials, Region},
    types::{BucketLocationConstraint, CreateBucketConfiguration},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Deserialize)]
pub struct S3ProviderCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
    pub endpoint_url: String,
    pub provider_region: Option<String>,
    pub provider_type: String,
}

#[derive(Serialize)]
struct S3AccessTokenSecret<'a> {
    secret_access_key: &'a str,
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

    pub async fn create_bucket(
        &self,
        provider_id: Uuid,
        bucket_name: &str,
    ) -> Result<(), AppError> {
        let provider = self.credentials(provider_id).await?;
        let region = provider
            .provider_region
            .clone()
            .unwrap_or_else(|| "us-east-1".into());
        let client = aws_sdk_s3::Client::from_conf(
            aws_sdk_s3::Config::builder()
                .behavior_version(BehaviorVersion::latest())
                .endpoint_url(provider.endpoint_url)
                .region(Region::new(region.clone()))
                .credentials_provider(Credentials::new(
                    provider.access_key_id,
                    provider.secret_access_key,
                    provider.session_token,
                    None,
                    "c-plane-control-plane",
                ))
                .force_path_style(true)
                .build(),
        );
        let request = client.create_bucket().bucket(bucket_name);
        let result = if provider.provider_type == "aws_s3" && region != "us-east-1" {
            request
                .create_bucket_configuration(
                    CreateBucketConfiguration::builder()
                        .location_constraint(BucketLocationConstraint::from(region.as_str()))
                        .build(),
                )
                .send()
                .await
        } else {
            request.send().await
        };
        result.map_err(|error| {
            tracing::error!(%error, %provider_id, bucket_name, "S3 provider bucket creation failed");
            AppError::Internal(format!("S3 provider bucket creation failed: {error}"))
        })?;
        Ok(())
    }

    pub async fn ensure_bucket_sse_key(&self, bucket_id: Uuid) -> Result<(), AppError> {
        let response = self
            .http
            .put(format!(
                "{}/internal/s3-buckets/{bucket_id}/sse-key",
                self.base_url
            ))
            .header("x-cplane-token", &self.service_token)
            .send()
            .await
            .map_err(|error| {
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

    pub async fn delete_bucket(
        &self,
        provider_id: Uuid,
        bucket_name: &str,
    ) -> Result<(), AppError> {
        let provider = self.credentials(provider_id).await?;
        let region = provider
            .provider_region
            .unwrap_or_else(|| "us-east-1".into());
        let client = aws_sdk_s3::Client::from_conf(
            aws_sdk_s3::Config::builder()
                .behavior_version(BehaviorVersion::latest())
                .endpoint_url(provider.endpoint_url)
                .region(Region::new(region))
                .credentials_provider(Credentials::new(
                    provider.access_key_id,
                    provider.secret_access_key,
                    provider.session_token,
                    None,
                    "c-plane-control-plane",
                ))
                .force_path_style(true)
                .build(),
        );
        client
            .delete_bucket()
            .bucket(bucket_name)
            .send()
            .await
            .map_err(|error| {
                tracing::error!(%error, %provider_id, bucket_name, "S3 provider bucket deletion failed");
                AppError::Conflict(format!("S3 provider bucket deletion failed: {error}"))
            })?;
        Ok(())
    }

    pub async fn store_access_token_secret(
        &self,
        credential_id: Uuid,
        secret_access_key: &str,
    ) -> Result<(), AppError> {
        self.secret_request(
            reqwest::Method::PUT,
            credential_id,
            Some(S3AccessTokenSecret { secret_access_key }),
        )
        .await
    }

    pub async fn delete_access_token_secret(&self, credential_id: Uuid) -> Result<(), AppError> {
        self.secret_request::<S3AccessTokenSecret<'_>>(reqwest::Method::DELETE, credential_id, None)
            .await
    }

    pub async fn invalidate_access_token_cache(&self) -> Result<(), AppError> {
        let response = self
            .http
            .delete(format!("{}/internal/s3-access-token-cache", self.base_url))
            .header("x-cplane-token", &self.service_token)
            .send()
            .await
            .map_err(|error| {
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

    async fn secret_request<T: Serialize>(
        &self,
        method: reqwest::Method,
        credential_id: Uuid,
        body: Option<T>,
    ) -> Result<(), AppError> {
        let mut request = self
            .http
            .request(
                method,
                format!(
                    "{}/internal/s3-access-tokens/{credential_id}",
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
