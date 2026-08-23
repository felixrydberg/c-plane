use axum::{Json, extract::Path};
use redis::AsyncCommands;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError,
    services::s3_providers::{S3AccessKeySecret, S3ProviderCredentials},
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
    pub bucket_permissions: Vec<ResolvedS3BucketPermission>,
    #[schema(ignore)]
    pub secret_access_key: String,
}

#[utoipa::path(
    get,
    path = "/internal/s3-access-tokens/resolve/{access_key}",
    params(("access_key" = String, Path)),
    responses(
        (status = 200, body = ResolvedS3AccessToken),
        (status = 401, body = crate::errors::ErrorResponse),
        (status = 404, body = crate::errors::ErrorResponse),
    ),
    security(("serviceToken" = [])),
    tag = "internal",
)]
pub async fn resolve_access_token(
    Path(access_key): Path<String>,
) -> Result<Json<ResolvedS3AccessToken>, AppError> {
    let state = get_app_state();
    let cached = cached(&state.config.redis_url, &access_key).await?;
    if let Some(cached) = cached {
        validate_cached_token(&cached, &access_key).await?;
        return Ok(Json(cached));
    }
    let secret = state
        .s3_providers
        .access_key(&access_key)
        .await?
        .ok_or_else(|| AppError::NotFound("S3 access key not found".into()))?;
    let resolved = resolve_uncached(secret, &access_key).await?;
    cache(&state.config.redis_url, &access_key, &resolved).await?;
    Ok(Json(resolved))
}

#[utoipa::path(
    get,
    path = "/internal/s3-providers/{provider_id}/credentials",
    params(("provider_id" = Uuid, Path)),
    responses(
        (status = 200, body = S3ProviderCredentials),
        (status = 401, body = crate::errors::ErrorResponse),
        (status = 404, body = crate::errors::ErrorResponse),
    ),
    security(("serviceToken" = [])),
    tag = "internal",
)]
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

async fn validate_cached_token(
    cached: &ResolvedS3AccessToken,
    access_key: &str,
) -> Result<(), AppError> {
    let state = get_app_state();
    let active = match (cached.organization_id, cached.project_id) {
        (Some(organization_id), Some(project_id)) => {
            let tenant = TenantDatabase::new(
                state.tenant_db,
                OrganizationContext {
                    allowed_organizations: vec![organization_id],
                    ..Default::default()
                },
            );
            let scoped = tenant.begin_scoped_transaction().await?;
            let row = scoped
                .connection()
                .query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT 1 FROM storage_access_token WHERE id=$1 AND access_key_id=$2 AND organization_id=$3 AND project_id=$4 AND revoked_at IS NULL",
                    vec![cached.credential_id.into(), access_key.into(), organization_id.into(), project_id.into()],
                ))
                .await?;
            scoped.commit().await?;
            row.is_some()
        }
        (None, None) => state
            .tenant_db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT 1 FROM registry_storage storage LEFT JOIN registry_maintenance maintenance ON maintenance.service=storage.service WHERE storage.id=$1 AND storage.service='distribution' AND (storage.access_key_id=$2 OR maintenance.gc_access_key_id=$2) LIMIT 1",
                vec![cached.credential_id.into(), access_key.into()],
            ))
            .await?
            .is_some(),
        _ => false,
    };
    if active {
        Ok(())
    } else {
        Err(AppError::Unauthorized("Invalid S3 access key".into()))
    }
}

async fn resolve_uncached(
    secret: S3AccessKeySecret,
    access_key: &str,
) -> Result<ResolvedS3AccessToken, AppError> {
    let state = get_app_state();
    let rows = match secret.kind.as_str() {
        "tenant" => {
            let organization_id = secret
                .organization_id
                .ok_or_else(|| AppError::Unauthorized("Invalid S3 access key".into()))?;
            let project_id = secret
                .project_id
                .ok_or_else(|| AppError::Unauthorized("Invalid S3 access key".into()))?;
            let tenant = TenantDatabase::new(
                state.tenant_db,
                OrganizationContext {
                    allowed_organizations: vec![organization_id],
                    ..Default::default()
                },
            );
            let scoped = tenant.begin_scoped_transaction().await?;
            let tx = scoped.connection();
            let active = tx
                .query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT id FROM storage_access_token WHERE id=$1 AND access_key_id=$2 AND organization_id=$3 AND project_id=$4 AND revoked_at IS NULL",
                    vec![secret.credential_id.into(), access_key.into(), organization_id.into(), project_id.into()],
                ))
                .await?;
            if active.is_none() {
                return Err(AppError::Unauthorized("Invalid S3 access key".into()));
            }
            let rows = tx
                .query_all(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT permission.bucket_id, permission.can_read, permission.can_write, bucket.name AS bucket_name, region.slug AS region_slug, provider.id AS provider_id, CONCAT('cp-', REPLACE(bucket.id::text, '-', '')) AS physical_bucket_name FROM storage_access_token_bucket permission JOIN bucket ON bucket.id=permission.bucket_id JOIN regions region ON region.id=bucket.region_id JOIN s3_providers provider ON provider.id=region.s3_provider_id WHERE permission.access_token_id=$1 AND bucket.project_id=$2 AND provider.is_active=true",
                    vec![secret.credential_id.into(), project_id.into()],
                ))
                .await?;
            scoped.commit().await?;
            rows
        }
        "distribution" | "distribution_gc" => state
            .tenant_db
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT storage.id AS bucket_id, true AS can_read, CASE WHEN $2='distribution_gc' THEN maintenance.phase='collecting' ELSE maintenance.phase IN ('idle', 'queued', 'draining') END AS can_write, storage.bucket_name, provider.provider_region AS region_slug, provider.id AS provider_id, storage.physical_bucket_name FROM registry_storage storage JOIN registry_maintenance maintenance ON maintenance.service=storage.service JOIN s3_providers provider ON provider.id=storage.provider_id WHERE storage.id=$1 AND storage.service='distribution' AND provider.is_active=true AND (($2='distribution' AND storage.access_key_id=$3) OR ($2='distribution_gc' AND maintenance.gc_access_key_id=$3))",
                vec![secret.credential_id.into(), secret.kind.clone().into(), access_key.into()],
            ))
            .await?,
        _ => return Err(AppError::Unauthorized("Invalid S3 access key".into())),
    };
    if rows.is_empty() {
        return Err(AppError::Unauthorized("Invalid S3 access key".into()));
    }
    let mut bucket_permissions = Vec::with_capacity(rows.len());
    for row in rows {
        let bucket_id: Uuid = row.try_get("", "bucket_id")?;
        bucket_permissions.push(ResolvedS3BucketPermission {
            bucket_id,
            bucket_name: row.try_get("", "bucket_name")?,
            physical_bucket_name: row.try_get("", "physical_bucket_name")?,
            region: row.try_get("", "region_slug")?,
            provider_id: row.try_get("", "provider_id")?,
            platform_sse_key: state.s3_providers.bucket_key(bucket_id).await?,
            can_read: row.try_get("", "can_read")?,
            can_write: row.try_get("", "can_write")?,
        });
    }
    Ok(ResolvedS3AccessToken {
        organization_id: secret.organization_id,
        project_id: secret.project_id,
        credential_id: secret.credential_id,
        bucket_permissions,
        secret_access_key: secret.secret_access_key,
    })
}

async fn connection(redis_url: &str) -> Result<redis::aio::MultiplexedConnection, AppError> {
    redis::Client::open(redis_url)
        .map_err(|error| AppError::Internal(error.to_string()))?
        .get_multiplexed_async_connection()
        .await
        .map_err(|error| AppError::Internal(error.to_string()))
}

async fn cached(
    redis_url: &str,
    access_key: &str,
) -> Result<Option<ResolvedS3AccessToken>, AppError> {
    let mut connection = connection(redis_url).await?;
    let value: Option<String> = connection
        .get(format!("{S3_ACCESS_TOKEN_CACHE_PREFIX}{access_key}"))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    value
        .map(|value| serde_json::from_str(&value))
        .transpose()
        .map_err(|error| AppError::Internal(error.to_string()))
}

async fn cache(
    redis_url: &str,
    access_key: &str,
    value: &ResolvedS3AccessToken,
) -> Result<(), AppError> {
    let mut connection = connection(redis_url).await?;
    let value =
        serde_json::to_string(value).map_err(|error| AppError::Internal(error.to_string()))?;
    connection
        .set_ex::<_, _, ()>(
            format!("{S3_ACCESS_TOKEN_CACHE_PREFIX}{access_key}"),
            value,
            CACHE_TTL_SECONDS,
        )
        .await
        .map_err(|error| AppError::Internal(error.to_string()))
}
