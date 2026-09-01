use crate::errors::AppError;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use lib::{
    buckets,
    cache::S3_PROVIDER_CREDENTIAL_CACHE_PREFIX,
    entities::{bucket, secret},
    secrets::{self, Client, PLATFORM_KEY},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, QueryFilter,
    Statement,
};
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
    pub name: String,
}

#[derive(Clone)]
pub struct S3ProviderClient {
    database: DatabaseConnection,
    secrets: Client,
    redis_url: String,
}

impl S3ProviderClient {
    pub fn new(database: DatabaseConnection, secrets: Client, redis_url: String) -> Self {
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
                "SELECT s3_providers.name, s3_providers.endpoint_url, s3_providers.provider_region, secret.ciphertext FROM s3_providers JOIN secret ON secret.id=s3_providers.credential_secret_id WHERE s3_providers.id=$1 AND s3_providers.is_active=true AND secret.scope='platform'::secret_scope AND secret.organization_id IS NULL",
                vec![provider_id.into()],
            ))
            .await?
            .ok_or_else(|| AppError::NotFound("S3 provider not found".into()))?;
        let ciphertext: String = row.try_get("", "ciphertext")?;
        let plaintext = secrets::decrypt(&self.secrets, PLATFORM_KEY, &ciphertext).await?;
        let secret: S3ProviderSecret = serde_json::from_slice(&plaintext)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let credentials = S3ProviderCredentials {
            access_key_id: secret.access_key_id,
            secret_access_key: secret.secret_access_key,
            session_token: secret.session_token,
            endpoint_url: row.try_get("", "endpoint_url")?,
            provider_region: row.try_get("", "provider_region")?,
            name: row.try_get("", "name")?,
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

    pub async fn bucket_key(
        &self,
        bucket_id: Uuid,
        organization_id: Option<Uuid>,
    ) -> Result<String, AppError> {
        let mut query = bucket::Entity::find_by_id(bucket_id).find_also_related(secret::Entity);
        query = match organization_id {
            Some(organization_id) => {
                query.filter(secret::Column::OrganizationId.eq(organization_id))
            }
            None => query.filter(secret::Column::OrganizationId.is_null()),
        };
        let secret = query
            .one(&self.database)
            .await?
            .and_then(|(_, secret)| secret)
            .ok_or_else(|| AppError::NotFound("Bucket encryption key not found".into()))?;
        let ciphertext = secret.ciphertext;
        let scope = secret.scope;
        let key = match (scope, organization_id) {
            (secret::SecretScope::Tenant, Some(organization_id)) => {
                format!("tenant-{}", organization_id.simple())
            }
            (secret::SecretScope::Platform, None) => PLATFORM_KEY.into(),
            _ => return Err(AppError::NotFound("Bucket encryption key not found".into())),
        };
        let plaintext = secrets::decrypt(&self.secrets, &key, &ciphertext).await?;
        String::from_utf8(plaintext)
            .map_err(|error| AppError::Internal(format!("Invalid bucket encryption key: {error}")))
    }

    pub async fn create_bucket(&self, provider_id: Uuid, bucket_id: Uuid) -> Result<(), AppError> {
        let provider = self.credentials(provider_id).await?;
        let region = provider
            .provider_region
            .clone()
            .unwrap_or_else(|| "us-east-1".into());
        let client = aws_sdk_s3::Client::from_conf(s3_config(&provider, &region));
        buckets::create(&client, Some(region.as_str()), bucket_id)
            .await
            .map_err(|error| {
                s3_bucket_operation_error(
                    provider_id,
                    bucket_id,
                    &provider,
                    &region,
                    "create",
                    &error,
                    AppError::Internal("S3 provider bucket creation failed".into()),
                )
            })
    }

    pub async fn delete_bucket(&self, provider_id: Uuid, bucket_id: Uuid) -> Result<(), AppError> {
        let provider = self.credentials(provider_id).await?;
        let region = provider
            .provider_region
            .clone()
            .unwrap_or_else(|| "us-east-1".into());
        let client = aws_sdk_s3::Client::from_conf(s3_config(&provider, &region));
        buckets::delete(&client, bucket_id).await.map_err(|error| {
            s3_bucket_operation_error(
                provider_id,
                bucket_id,
                &provider,
                &region,
                "delete",
                &error,
                AppError::Conflict("S3 provider bucket deletion failed".into()),
            )
        })
    }

    pub async fn bucket_is_empty(
        &self,
        provider_id: Uuid,
        bucket_id: Uuid,
    ) -> Result<bool, AppError> {
        let provider = self.credentials(provider_id).await?;
        let region = provider
            .provider_region
            .clone()
            .unwrap_or_else(|| "us-east-1".into());
        let client = aws_sdk_s3::Client::from_conf(s3_config(&provider, &region));
        lib::buckets::is_empty(&client, bucket_id)
            .await
            .map_err(|error| {
                tracing::error!(%provider_id, %error, "S3 provider bucket status check failed");
                AppError::Conflict("S3 provider bucket status could not be checked".into())
            })
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

fn s3_bucket_operation_error(
    provider_id: Uuid,
    bucket_id: Uuid,
    provider: &S3ProviderCredentials,
    region: &str,
    operation: &'static str,
    error: &lib::buckets::Error,
    fallback: AppError,
) -> AppError {
    let (error_code, error_message) = buckets::error_details(error);
    let credentials_error = buckets::is_credentials_error(error);
    tracing::error!(
        %provider_id,
        %bucket_id,
        provider_name = %provider.name,
        %region,
        operation,
        error_code = error_code.unwrap_or("unknown"),
        error_message = error_message.unwrap_or("unknown"),
        error = %error,
        credentials_error,
        "S3 provider bucket operation failed"
    );

    if credentials_error {
        AppError::ServiceUnavailable("S3 provider credentials are invalid".into())
    } else {
        fallback
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
