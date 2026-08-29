use axum::{Json, extract::Path};
use redis::AsyncCommands;
use sea_orm::{
    ColumnTrait, EntityTrait, FromQueryResult, JoinType, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::entities::{
        bucket, bucket_grant, credential, region, s3_provider, secret, storage,
        storage_access_token,
    },
    services::s3_providers::S3ProviderCredentials,
    state::{OrganizationContext, TenantDatabase, get_app_state},
};
use lib::cache::S3_ACCESS_TOKEN_CACHE_PREFIX;
use lib::entities::secret::SecretScope;

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
    #[serde(default)]
    pub is_deleting: bool,
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct ResolvedS3AccessToken {
    pub organization_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
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
    storage_bucket_id: Uuid,
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
    let organization_id = credential
        .organization_id
        .ok_or_else(|| AppError::Internal("S3 credential has no organization".into()))?;
    let token = storage_access_token::Entity::find_by_id(credential.id)
        .filter(storage_access_token::Column::OrganizationId.eq(organization_id))
        .one(state.identity_db.connection())
        .await?
        .ok_or_else(|| AppError::NotFound("S3 access token not found".into()))?;
    let secret = secret::Entity::find_by_id(credential.secret_id)
        .filter(secret::Column::Scope.eq(SecretScope::Tenant))
        .filter(secret::Column::OrganizationId.eq(organization_id))
        .one(state.identity_db.connection())
        .await?
        .ok_or_else(|| AppError::NotFound("S3 access token secret not found".into()))?;
    let plaintext = lib::secrets::decrypt(
        &state.secrets,
        &format!("tenant-{}", organization_id.simple()),
        &secret.ciphertext,
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
    let tx = scoped.connection();
    let rows = storage::Entity::find()
        .select_only()
        .column_as(storage::Column::Id, "storage_bucket_id")
        .column_as(storage::Column::Name, "bucket_name")
        .column_as(bucket::Column::Id, "foundation_bucket_id")
        .column_as(region::Column::Slug, "region")
        .column_as(region::Column::S3ProviderId, "provider_id")
        .column(bucket_grant::Column::CanRead)
        .column(bucket_grant::Column::CanWrite)
        .column(bucket::Column::Status)
        .inner_join(bucket::Entity)
        .join(JoinType::InnerJoin, bucket::Relation::Region.def())
        .join(JoinType::InnerJoin, region::Relation::S3Provider.def())
        .inner_join(bucket_grant::Entity)
        .filter(storage::Column::OrganizationId.eq(organization_id))
        .filter(storage::Column::ProjectId.eq(token.project_id))
        .filter(bucket_grant::Column::CredentialId.eq(credential.id))
        .filter(bucket_grant::Column::OrganizationId.eq(organization_id))
        .filter(s3_provider::Column::IsActive.eq(true))
        .order_by_asc(storage::Column::Name)
        .into_model::<S3BucketPermissionRow>()
        .all(tx)
        .await?;
    scoped.commit().await?;
    let mut bucket_permissions = Vec::with_capacity(rows.len());
    for row in rows {
        bucket_permissions.push(ResolvedS3BucketPermission {
            bucket_id: row.storage_bucket_id,
            bucket_name: row.bucket_name,
            physical_bucket_name: lib::buckets::physical_bucket_name(row.foundation_bucket_id),
            region: row.region,
            provider_id: row.provider_id,
            platform_sse_key: state
                .s3_providers
                .bucket_key(row.foundation_bucket_id, organization_id)
                .await?,
            can_read: row.can_read,
            can_write: row.can_write,
            is_deleting: row.status == bucket::BucketStatus::Deleting,
        });
    }
    Ok(ResolvedS3AccessToken {
        organization_id: Some(organization_id),
        project_id: Some(token.project_id),
        credential_id: credential.id,
        prefix: credential.prefix,
        bucket_permissions,
        secret_access_key: secret.secret_access_key,
    })
}

async fn validate_cached_token(
    cached: &ResolvedS3AccessToken,
    access_key: &str,
) -> Result<bool, AppError> {
    let (Some(organization_id), Some(project_id)) = (cached.organization_id, cached.project_id)
    else {
        return Err(AppError::Unauthorized("Invalid S3 access key".into()));
    };
    let state = get_app_state();
    let active = storage_access_token::Entity::find()
        .filter(storage_access_token::Column::CredentialId.eq(cached.credential_id))
        .filter(storage_access_token::Column::OrganizationId.eq(organization_id))
        .filter(storage_access_token::Column::ProjectId.eq(project_id))
        .join(
            JoinType::InnerJoin,
            storage_access_token::Relation::Credential.def(),
        )
        .filter(credential::Column::AccessKeyId.eq(access_key))
        .filter(credential::Column::OrganizationId.eq(organization_id))
        .filter(credential::Column::RevokedAt.is_null())
        .one(state.identity_db.connection())
        .await?
        .is_some();
    if !active {
        return Err(AppError::Unauthorized("Invalid S3 access key".into()));
    }

    let tenant = TenantDatabase::new(
        state.tenant_db,
        OrganizationContext {
            allowed_organizations: vec![organization_id],
            ..Default::default()
        },
    );
    let scoped = tenant.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let deleting = storage::Entity::find()
        .join(JoinType::InnerJoin, storage::Relation::Bucket.def())
        .join(JoinType::InnerJoin, storage::Relation::BucketGrant.def())
        .filter(storage::Column::OrganizationId.eq(organization_id))
        .filter(storage::Column::ProjectId.eq(project_id))
        .filter(bucket_grant::Column::CredentialId.eq(cached.credential_id))
        .filter(bucket_grant::Column::OrganizationId.eq(organization_id))
        .filter(bucket::Column::Status.eq(bucket::BucketStatus::Deleting))
        .one(tx)
        .await?
        .is_some();
    scoped.commit().await?;
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
