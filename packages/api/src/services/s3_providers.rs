use crate::errors::AppError;
use aws_sdk_s3::{
    config::{BehaviorVersion, Credentials, Region},
    types::{BucketLocationConstraint, CreateBucketConfiguration},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use lib::{cache::S3_PROVIDER_CREDENTIAL_CACHE_PREFIX, secrets::Secrets};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

const PROVIDER_CREDENTIAL_CACHE_TTL_SECONDS: u64 = 60;

#[derive(Clone, Deserialize, Serialize)]
pub struct S3ProviderSecret {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct S3ProviderCredentials {
    pub access_key_id: String,
    #[schema(ignore)]
    pub secret_access_key: String,
    #[schema(ignore)]
    pub session_token: Option<String>,
    pub endpoint_url: String,
    pub provider_region: Option<String>,
    pub provider_type: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct S3AccessKeySecret {
    pub kind: String,
    pub credential_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub secret_access_key: String,
}

#[derive(Deserialize, Serialize)]
struct BucketKey {
    key: String,
}

#[derive(Clone)]
pub struct S3ProviderClient {
    database: DatabaseConnection,
    secrets: Secrets,
    redis_url: String,
}

impl S3ProviderClient {
    pub fn new(database: DatabaseConnection, secrets: Secrets, redis_url: String) -> Self {
        Self {
            database,
            secrets,
            redis_url,
        }
    }

    pub async fn credentials(&self, provider_id: Uuid) -> Result<S3ProviderCredentials, AppError> {
        let client = redis::Client::open(self.redis_url.as_str())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let mut connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let cache_key = format!("{S3_PROVIDER_CREDENTIAL_CACHE_PREFIX}{provider_id}");
        let cached = redis::cmd("GET")
            .arg(&cache_key)
            .query_async::<Option<String>>(&mut connection)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;
        if let Some(cached) = cached {
            return serde_json::from_str(&cached)
                .map_err(|error| AppError::Internal(error.to_string()));
        }

        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT endpoint_url, provider_region, provider_type::text FROM s3_providers WHERE id=$1 AND is_active=true",
                vec![provider_id.into()],
            ))
            .await?
            .ok_or_else(|| AppError::NotFound("S3 provider not found".into()))?;
        let secret = self
            .secrets
            .get::<S3ProviderSecret>(&format!("platform/s3/providers/{provider_id}"))
            .await?
            .ok_or_else(|| AppError::NotFound("S3 provider credentials not found".into()))?;
        let credentials = S3ProviderCredentials {
            access_key_id: secret.access_key_id,
            secret_access_key: secret.secret_access_key,
            session_token: secret.session_token,
            endpoint_url: row.try_get("", "endpoint_url")?,
            provider_region: row.try_get("", "provider_region")?,
            provider_type: row.try_get("", "provider_type")?,
        };
        redis::cmd("SETEX")
            .arg(cache_key)
            .arg(PROVIDER_CREDENTIAL_CACHE_TTL_SECONDS)
            .arg(
                serde_json::to_string(&credentials)
                    .map_err(|error| AppError::Internal(error.to_string()))?,
            )
            .query_async::<()>(&mut connection)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;
        Ok(credentials)
    }

    pub async fn access_key(
        &self,
        access_key: &str,
    ) -> Result<Option<S3AccessKeySecret>, AppError> {
        Ok(self
            .secrets
            .get(&format!("platform/s3/access-keys/{access_key}"))
            .await?)
    }

    pub async fn store_access_key(
        &self,
        access_key: &str,
        secret: &S3AccessKeySecret,
    ) -> Result<(), AppError> {
        self.secrets
            .set(&format!("platform/s3/access-keys/{access_key}"), secret)
            .await?;
        Ok(())
    }

    pub async fn delete_access_key(&self, access_key: &str) -> Result<(), AppError> {
        self.invalidate_access_token_cache(access_key).await?;
        self.secrets
            .delete(&format!("platform/s3/access-keys/{access_key}"))
            .await?;
        Ok(())
    }

    pub async fn bucket_key(&self, bucket_id: Uuid) -> Result<String, AppError> {
        self.secrets
            .get::<BucketKey>(&format!("storage/sse-c/{bucket_id}"))
            .await?
            .map(|secret| secret.key)
            .ok_or_else(|| AppError::NotFound("Bucket encryption key not found".into()))
    }

    pub async fn ensure_bucket_sse_key(&self, bucket_id: Uuid) -> Result<(), AppError> {
        let path = format!("storage/sse-c/{bucket_id}");
        if self.secrets.get::<BucketKey>(&path).await?.is_none() {
            let mut raw = [0; 32];
            getrandom::fill(&mut raw).map_err(|error| {
                AppError::Internal(format!("Failed to generate bucket key: {error}"))
            })?;
            self.secrets
                .set(
                    &path,
                    &BucketKey {
                        key: STANDARD.encode(raw),
                    },
                )
                .await?;
        }
        Ok(())
    }

    pub async fn delete_bucket_sse_key(&self, bucket_id: Uuid) -> Result<(), AppError> {
        self.secrets
            .delete(&format!("storage/sse-c/{bucket_id}"))
            .await?;
        Ok(())
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
        let client = aws_sdk_s3::Client::from_conf(s3_config(&provider, &region));
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
        result.map_err(|_error| {
            tracing::error!(%provider_id, "S3 provider bucket creation failed");
            AppError::Internal("S3 provider bucket creation failed".into())
        })?;
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
            .clone()
            .unwrap_or_else(|| "us-east-1".into());
        aws_sdk_s3::Client::from_conf(s3_config(&provider, &region))
            .delete_bucket()
            .bucket(bucket_name)
            .send()
            .await
            .map_err(|_error| {
                tracing::error!(%provider_id, "S3 provider bucket deletion failed");
                AppError::Conflict("S3 provider bucket deletion failed".into())
            })?;
        Ok(())
    }

    pub async fn invalidate_access_token_cache(&self, access_key: &str) -> Result<(), AppError> {
        self.invalidate_access_token_caches(&[access_key.to_owned()])
            .await
    }

    pub async fn invalidate_access_token_caches(
        &self,
        access_keys: &[String],
    ) -> Result<(), AppError> {
        if access_keys.is_empty() {
            return Ok(());
        }
        let client = redis::Client::open(self.redis_url.as_str())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let mut connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;
        redis::cmd("DEL")
            .arg(
                access_keys
                    .iter()
                    .map(|access_key| {
                        format!("{}{}", lib::cache::S3_ACCESS_TOKEN_CACHE_PREFIX, access_key)
                    })
                    .collect::<Vec<_>>(),
            )
            .query_async::<u64>(&mut connection)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;
        Ok(())
    }
}

fn s3_config(provider: &S3ProviderCredentials, region: &str) -> aws_sdk_s3::Config {
    aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .endpoint_url(provider.endpoint_url.clone())
        .region(Region::new(region.to_owned()))
        .credentials_provider(Credentials::new(
            provider.access_key_id.clone(),
            provider.secret_access_key.clone(),
            provider.session_token.clone(),
            None,
            "c-plane-api",
        ))
        .force_path_style(true)
        .build()
}
