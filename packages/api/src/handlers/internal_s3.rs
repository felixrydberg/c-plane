use axum::{Json, extract::Path};
use redis::AsyncCommands;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError,
    services::s3_providers::S3ProviderCredentials,
    state::{OrganizationContext, TenantDatabase, get_app_state},
};
use lib::cache::S3_ACCESS_TOKEN_CACHE_PREFIX;

const CACHE_TTL_SECONDS: u64 = 86_400;

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct ResolvedS3BucketPermission {
    pub bucket_id: Uuid,
    pub bucket_name: String,
    pub physical_bucket_name: String,
    pub region: String,
    pub provider_id: Uuid,
    #[schema(ignore)]
    pub platform_sse_key: String,
    pub can_read: bool,
    pub can_write: bool,
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct ResolvedS3AccessToken {
    pub organization_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub credential_id: Uuid,
    #[serde(default)]
    pub prefix: String,
    pub bucket_permissions: Vec<ResolvedS3BucketPermission>,
    #[schema(ignore)]
    pub secret_access_key: String,
}

#[utoipa::path(get, path = "/internal/s3-access-tokens/resolve/{access_key}", params(("access_key" = String, Path)),
    responses((status = 200, body = ResolvedS3AccessToken), (status = 401, body = crate::errors::ErrorResponse), (status = 404, body = crate::errors::ErrorResponse)), security(("serviceToken" = [])), tag = "internal")]
pub async fn resolve_access_token(
    Path(access_key): Path<String>,
) -> Result<Json<ResolvedS3AccessToken>, AppError> {
    let state = get_app_state();
    if let Some(cached) = cached(&state.config.redis_url, &access_key).await? {
        validate_cached_token(&cached, &access_key).await?;
        return Ok(Json(cached));
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
    let credential = state
        .identity_db
        .connection()
        .query_one(statement(
        "SELECT credential.id, credential.organization_id, credential.prefix, token.project_id, secret.ciphertext FROM credential JOIN storage_access_token token ON token.credential_id=credential.id JOIN secret ON secret.id=credential.secret_id WHERE credential.access_key_id=$1 AND credential.revoked_at IS NULL AND secret.scope='tenant'::secret_scope AND secret.organization_id=credential.organization_id",
        vec![access_key.into()],
    ))
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?
    .ok_or_else(|| AppError::NotFound("S3 access key not found".into()))?;
    let credential_id: Uuid = credential.try_get("", "id")?;
    let organization_id: Uuid = credential.try_get("", "organization_id")?;
    let prefix: String = credential.try_get("", "prefix")?;
    let project_id: Uuid = credential.try_get("", "project_id")?;
    let ciphertext: String = credential.try_get("", "ciphertext")?;
    let plaintext = lib::secrets::decrypt(
        &state.secrets,
        &format!("tenant-{}", organization_id.simple()),
        &ciphertext,
    )
    .await?;
    let secret: S3SecretKey = serde_json::from_slice(&plaintext)
        .map_err(|error| AppError::Internal(error.to_string()))?;

    let tenant = TenantDatabase::new(
        state.tenant_db,
        OrganizationContext {
            allowed_organizations: vec![organization_id],
            ..Default::default()
        },
    );
    let scoped = tenant.begin_scoped_transaction().await?;
    let rows = scoped.connection().query_all(statement(
        "SELECT storage_bucket.id AS storage_bucket_id, storage_bucket.name AS bucket_name, bucket.id AS foundation_bucket_id, regions.slug AS region_slug, provider.id AS provider_id, bg.can_read, bg.can_write FROM bucket_grant bg JOIN storage_bucket ON storage_bucket.bucket_id=bg.bucket_id JOIN bucket ON bucket.id=bg.bucket_id JOIN regions ON regions.id=bucket.region_id JOIN s3_providers provider ON provider.id=regions.s3_provider_id WHERE bg.credential_id=$1 AND bg.organization_id=$2 AND storage_bucket.project_id=$3 AND provider.is_active=true ORDER BY storage_bucket.name",
        vec![credential_id.into(), organization_id.into(), project_id.into()],
    )).await?;
    scoped.commit().await?;
    let mut bucket_permissions = Vec::with_capacity(rows.len());
    for row in rows {
        let foundation_bucket_id: Uuid = row.try_get("", "foundation_bucket_id")?;
        bucket_permissions.push(ResolvedS3BucketPermission {
            bucket_id: row.try_get("", "storage_bucket_id")?,
            bucket_name: row.try_get("", "bucket_name")?,
            physical_bucket_name: lib::buckets::physical_bucket_name(foundation_bucket_id),
            region: row.try_get("", "region_slug")?,
            provider_id: row.try_get("", "provider_id")?,
            platform_sse_key: state
                .s3_providers
                .bucket_key(foundation_bucket_id, organization_id)
                .await?,
            can_read: row.try_get("", "can_read")?,
            can_write: row.try_get("", "can_write")?,
        });
    }
    Ok(ResolvedS3AccessToken {
        organization_id: Some(organization_id),
        project_id: Some(project_id),
        credential_id,
        prefix,
        bucket_permissions,
        secret_access_key: secret.secret_access_key,
    })
}

async fn validate_cached_token(
    cached: &ResolvedS3AccessToken,
    access_key: &str,
) -> Result<(), AppError> {
    let (Some(organization_id), Some(project_id)) = (cached.organization_id, cached.project_id)
    else {
        return Err(AppError::Unauthorized("Invalid S3 access key".into()));
    };
    let state = get_app_state();
    let tenant = TenantDatabase::new(
        state.tenant_db,
        OrganizationContext {
            allowed_organizations: vec![organization_id],
            ..Default::default()
        },
    );
    let scoped = tenant.begin_scoped_transaction().await?;
    let active = scoped.connection().query_one(statement(
        "SELECT 1 FROM storage_access_token token JOIN credential ON credential.id=token.credential_id WHERE token.credential_id=$1 AND credential.access_key_id=$2 AND token.organization_id=$3 AND token.project_id=$4 AND credential.revoked_at IS NULL",
        vec![cached.credential_id.into(), access_key.into(), organization_id.into(), project_id.into()],
    )).await?.is_some();
    scoped.commit().await?;
    active
        .then_some(())
        .ok_or_else(|| AppError::Unauthorized("Invalid S3 access key".into()))
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

fn statement(sql: &str, values: Vec<sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}
