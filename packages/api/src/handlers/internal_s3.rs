use axum::{Json, extract::Path};
use redis::AsyncCommands;
use sea_orm::{
    ColumnTrait, DatabaseBackend, EntityTrait, FromQueryResult, JoinType, QueryFilter, QuerySelect,
    RelationTrait, Statement,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::entities::{bucket, bucket_grant, credential, secret},
    services::s3_providers::S3ProviderCredentials,
    state::get_app_state,
};
use lib::{
    cache::S3_ACCESS_TOKEN_CACHE_PREFIX, entities::secret::SecretScope, secrets::PLATFORM_KEY,
};

const CACHE_TTL_SECONDS: u64 = 86_400;

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct ResolvedS3BucketPermission {
    /// Foundation bucket ID.
    pub bucket_id: Uuid,
    /// Logical S3 bucket name presented to the credential.
    pub bucket_name: String,
    pub physical_bucket_name: String,
    pub region: String,
    pub provider_id: Uuid,
    #[schema(ignore)]
    pub platform_sse_key: String,
    pub can_read: bool,
    pub can_write: bool,
    #[serde(default)]
    #[schema(required)]
    pub is_deleting: bool,
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct ResolvedS3AccessToken {
    pub organization_id: Option<Uuid>,
    pub credential_id: Uuid,
    #[serde(default)]
    #[schema(required)]
    pub prefix: String,
    pub bucket_permissions: Vec<ResolvedS3BucketPermission>,
    #[schema(ignore)]
    pub secret_access_key: String,
}

#[derive(FromQueryResult)]
struct S3BucketPermissionRow {
    bucket_name: String,
    foundation_bucket_id: Uuid,
    region: String,
    provider_id: Uuid,
    can_read: bool,
    can_write: bool,
    status: bucket::BucketStatus,
}

#[utoipa::path(get, path = "/internal/s3-access-tokens/resolve/{access_key}", params(("access_key" = String, Path)),
    responses((status = 200, body = ResolvedS3AccessToken), (status = 401, body = crate::errors::ErrorResponse), (status = 404, body = crate::errors::ErrorResponse)), security(("serviceToken" = [])), tag = "internal")]
pub async fn resolve_access_token(
    Path(access_key): Path<String>,
) -> Result<Json<ResolvedS3AccessToken>, AppError> {
    let state = get_app_state();
    if let Some(cached) = cached(&state.config.redis_url, &access_key).await? {
        if validate_cached_token(&cached, &access_key).await? {
            return Ok(Json(cached));
        }
    }
    let resolved = resolve_uncached(&access_key).await?;
    cache(&state.config.redis_url, &access_key, &resolved).await?;
    Ok(Json(resolved))
}

#[utoipa::path(get, path = "/internal/s3-providers/{provider_id}/credentials", params(("provider_id" = Uuid, Path)),
    responses((status = 200, body = S3ProviderCredentials), (status = 401, body = crate::errors::ErrorResponse), (status = 404, body = crate::errors::ErrorResponse)), security(("serviceToken" = [])), tag = "internal")]
pub async fn provider_credentials(
    Path(provider_id): Path<Uuid>,
) -> Result<Json<S3ProviderCredentials>, AppError> {
    Ok(Json(
        get_app_state()
            .s3_providers
            .credentials(provider_id)
            .await?,
    ))
}

async fn resolve_uncached(access_key: &str) -> Result<ResolvedS3AccessToken, AppError> {
    let state = get_app_state();
    let credential = credential::Entity::find()
        .filter(credential::Column::AccessKeyId.eq(access_key))
        .filter(credential::Column::RevokedAt.is_null())
        .one(state.identity_db.connection())
        .await?
        .ok_or_else(|| AppError::NotFound("S3 access key not found".into()))?;
    let credential_secret = secret::Entity::find_by_id(credential.secret_id)
        .one(state.identity_db.connection())
        .await?
        .ok_or_else(|| AppError::NotFound("S3 credential secret not found".into()))?;
    let transit_key = match (
        credential.organization_id,
        credential_secret.scope,
        credential_secret.organization_id,
    ) {
        (Some(organization_id), SecretScope::Tenant, Some(secret_organization_id))
            if organization_id == secret_organization_id =>
        {
            format!("tenant-{}", organization_id.simple())
        }
        (None, SecretScope::Platform, None) => PLATFORM_KEY.into(),
        _ => return Err(AppError::Unauthorized("Invalid S3 credential scope".into())),
    };
    let plaintext =
        lib::secrets::decrypt(&state.secrets, &transit_key, &credential_secret.ciphertext).await?;
    let s3_secret: S3SecretKey = serde_json::from_slice(&plaintext)
        .map_err(|error| AppError::Internal(error.to_string()))?;

    let rows = S3BucketPermissionRow::find_by_statement(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT grants.bucket_id AS foundation_bucket_id, COALESCE(storage.name, CASE WHEN managed.organization_id IS NOT NULL THEN 'registry' END) AS bucket_name, regions.slug AS region, regions.s3_provider_id AS provider_id, grants.can_read, grants.can_write, foundation.status::text AS status FROM bucket_grant grants JOIN bucket foundation ON foundation.id=grants.bucket_id JOIN regions ON regions.id=foundation.region_id JOIN s3_providers provider ON provider.id=regions.s3_provider_id LEFT JOIN storage_bucket storage ON storage.bucket_id=grants.bucket_id AND storage.organization_id IS NOT DISTINCT FROM grants.organization_id LEFT JOIN managed_registry managed ON managed.bucket_id=grants.bucket_id AND managed.organization_id IS NOT DISTINCT FROM grants.organization_id WHERE grants.credential_id=$1 AND provider.is_active=true AND (storage.id IS NOT NULL OR managed.organization_id IS NOT NULL) ORDER BY bucket_name",
        vec![credential.id.into()],
    ))
        .all(state.identity_db.connection())
        .await?;

    let mut bucket_permissions = Vec::with_capacity(rows.len());
    for row in rows {
        bucket_permissions.push(ResolvedS3BucketPermission {
            bucket_id: row.foundation_bucket_id,
            bucket_name: row.bucket_name,
            physical_bucket_name: lib::buckets::physical_bucket_name(row.foundation_bucket_id),
            region: row.region,
            provider_id: row.provider_id,
            platform_sse_key: state
                .s3_providers
                .bucket_key(row.foundation_bucket_id, credential.organization_id)
                .await?,
            can_read: row.can_read,
            can_write: row.can_write,
            is_deleting: row.status == bucket::BucketStatus::Deleting,
        });
    }
    Ok(ResolvedS3AccessToken {
        organization_id: credential.organization_id,
        credential_id: credential.id,
        prefix: credential.prefix,
        bucket_permissions,
        secret_access_key: s3_secret.secret_access_key,
    })
}

async fn validate_cached_token(
    cached: &ResolvedS3AccessToken,
    access_key: &str,
) -> Result<bool, AppError> {
    let state = get_app_state();
    let active = credential::Entity::find_by_id(cached.credential_id)
        .filter(credential::Column::AccessKeyId.eq(access_key))
        .filter(credential::Column::RevokedAt.is_null())
        .one(state.identity_db.connection())
        .await?
        .is_some_and(|credential| credential.organization_id == cached.organization_id);
    if !active {
        return Err(AppError::Unauthorized("Invalid S3 access key".into()));
    }
    let deleting = bucket_grant::Entity::find()
        .join(JoinType::InnerJoin, bucket_grant::Relation::Bucket.def())
        .filter(bucket_grant::Column::CredentialId.eq(cached.credential_id))
        .filter(bucket::Column::Status.eq(bucket::BucketStatus::Deleting))
        .one(state.identity_db.connection())
        .await?
        .is_some();
    Ok(!deleting)
}

async fn cached(
    redis_url: &str,
    access_key: &str,
) -> Result<Option<ResolvedS3AccessToken>, AppError> {
    let client =
        redis::Client::open(redis_url).map_err(|error| AppError::Internal(error.to_string()))?;
    let mut connection = client
        .get_multiplexed_async_connection()
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let cached = redis::cmd("GET")
        .arg(format!("{S3_ACCESS_TOKEN_CACHE_PREFIX}{access_key}"))
        .query_async::<Option<String>>(&mut connection)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    cached
        .map(|value| {
            serde_json::from_str(&value).map_err(|error| AppError::Internal(error.to_string()))
        })
        .transpose()
}

async fn cache(
    redis_url: &str,
    access_key: &str,
    value: &ResolvedS3AccessToken,
) -> Result<(), AppError> {
    let client =
        redis::Client::open(redis_url).map_err(|error| AppError::Internal(error.to_string()))?;
    let mut connection = client
        .get_multiplexed_async_connection()
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    connection
        .set_ex::<_, _, ()>(
            format!("{S3_ACCESS_TOKEN_CACHE_PREFIX}{access_key}"),
            serde_json::to_string(value).map_err(|error| AppError::Internal(error.to_string()))?,
            CACHE_TTL_SECONDS,
        )
        .await
        .map_err(|error| AppError::Internal(error.to_string()))
}

#[derive(Deserialize)]
struct S3SecretKey {
    secret_access_key: String,
}
