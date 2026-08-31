use axum::{Json, extract::Path};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, JoinType, QueryFilter,
    QueryOrder, QuerySelect, RelationTrait, Set,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use utoipa::ToSchema;
use uuid::Uuid;

use super::databases::{verify_org_access, verify_org_owner, verify_project_in_org};
use crate::{
    errors::AppError,
    middleware::auth::AuthContext,
    models::entities::secret::SecretScope,
    models::entities::{bucket_grant, credential, secret, storage, storage_access_token},
    services::buckets::tenant_key,
    state::get_app_state,
};

const ACCESS_KEY_PREFIX: &str = "CP";
const MAX_CREDENTIAL_PREFIX_BYTES: usize = 1024;

#[derive(Deserialize, ToSchema)]
pub struct CreateAccessTokenRequest {
    pub name: String,
    #[serde(default)]
    pub prefix: String,
    pub bucket_permissions: Vec<BucketPermissionRequest>,
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct BucketPermissionRequest {
    pub bucket_id: Uuid,
    pub can_read: bool,
    pub can_write: bool,
}

#[derive(Serialize, ToSchema)]
pub struct AccessTokenResponse {
    pub id: Uuid,
    pub name: String,
    pub access_key_id: String,
    pub prefix: String,
    pub created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct CreatedAccessTokenResponse {
    #[serde(flatten)]
    pub token: AccessTokenResponse,
    pub secret_access_key: String,
    pub endpoint_url: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateAccessTokenRequest {
    pub bucket_permissions: Vec<BucketPermissionRequest>,
}

#[derive(Serialize, ToSchema)]
pub struct AccessTokenDetailsResponse {
    #[serde(flatten)]
    pub token: AccessTokenResponse,
    pub bucket_permissions: Vec<BucketPermissionRequest>,
}

#[derive(Serialize)]
struct S3SecretKey {
    secret_access_key: String,
}

#[utoipa::path(post, operation_id = "storage_create_access_token", path = "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens", request_body = CreateAccessTokenRequest,
    params(("organization_id" = Uuid, Path, description = "Organization ID"), ("project_id" = Uuid, Path, description = "Project ID")),
    responses((status = 201, description = "Access token created; save the secret access key now", body = CreatedAccessTokenResponse), (status = 404, description = "Project or bucket not found"), (status = 409, description = "Invalid permissions or duplicate token name")), tag = "storage")]
pub async fn create_access_token(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateAccessTokenRequest>,
) -> Result<(axum::http::StatusCode, Json<CreatedAccessTokenResponse>), AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    verify_org_owner(&tenant_db, organization_id)?;
    let name = body.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::Conflict(
            "Token name must be 1-100 characters".into(),
        ));
    }
    if !valid_credential_prefix(&body.prefix) {
        return Err(AppError::Conflict(
            "Credential prefix must be at most 1024 bytes".into(),
        ));
    }
    if !valid_bucket_permissions(&body.bucket_permissions) {
        return Err(AppError::Conflict("Invalid bucket permissions".into()));
    }

    let state = get_app_state();
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, project_id, organization_id).await?;
    let foundation_bucket_ids =
        verify_bucket_permissions(tx, organization_id, project_id, &body.bucket_permissions)
            .await?;
    if active_token_named(tx, project_id, name).await? {
        return Err(AppError::Conflict(
            "A token with this name already exists".into(),
        ));
    }

    let credential_id = Uuid::new_v4();
    let secret_id = Uuid::new_v4();
    let access_key_id = format!("{ACCESS_KEY_PREFIX}{}", Uuid::new_v4().simple()).to_uppercase();
    let secret_access_key = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let plaintext = serde_json::to_vec(&S3SecretKey {
        secret_access_key: secret_access_key.clone(),
    })
    .map_err(|error| AppError::Internal(error.to_string()))?;
    let ciphertext =
        lib::secrets::encrypt(&state.secrets, &tenant_key(organization_id), &plaintext).await?;
    secret::ActiveModel {
        id: Set(secret_id),
        scope: Set(SecretScope::Tenant),
        organization_id: Set(Some(organization_id)),
        ciphertext: Set(ciphertext),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    credential::ActiveModel {
        id: Set(credential_id),
        organization_id: Set(Some(organization_id)),
        access_key_id: Set(access_key_id.clone()),
        prefix: Set(body.prefix.clone()),
        secret_id: Set(secret_id),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    storage_access_token::ActiveModel {
        credential_id: Set(credential_id),
        organization_id: Set(organization_id),
        project_id: Set(project_id),
        name: Set(name.into()),
    }
    .insert(tx)
    .await?;
    insert_grants(
        tx,
        credential_id,
        organization_id,
        &foundation_bucket_ids,
        &body.bucket_permissions,
    )
    .await?;
    let token = token_by_id(tx, organization_id, project_id, credential_id).await?;
    crate::services::events::record(tx, organization_id, project_id, "storage-access-token:created", serde_json::json!({
        "summary": format!("Created storage access token '{name}'"), "target_id": credential_id.to_string(),
        "bucket_ids": body.bucket_permissions.iter().map(|permission| permission.bucket_id).collect::<Vec<_>>(),
    }), auth.actor_id).await?;
    scoped.commit().await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(CreatedAccessTokenResponse {
            token,
            secret_access_key,
            endpoint_url: state.config.storage_endpoint_url,
        }),
    ))
}

#[utoipa::path(get, operation_id = "storage_list_access_tokens", path = "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens",
    params(("organization_id" = Uuid, Path, description = "Organization ID"), ("project_id" = Uuid, Path, description = "Project ID")),
    responses((status = 200, description = "List of active access tokens", body = Vec<AccessTokenResponse>), (status = 404, description = "Project not found")), tag = "storage")]
pub async fn list_access_tokens(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<AccessTokenResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, project_id, organization_id).await?;
    let rows = storage_access_token::Entity::find()
        .find_also_related(credential::Entity)
        .filter(storage_access_token::Column::OrganizationId.eq(organization_id))
        .filter(storage_access_token::Column::ProjectId.eq(project_id))
        .filter(credential::Column::RevokedAt.is_null())
        .order_by_desc(credential::Column::CreatedAt)
        .all(tx)
        .await?;
    let tokens = rows
        .into_iter()
        .filter_map(|(token, credential)| {
            credential.map(|credential| AccessTokenResponse {
                id: token.credential_id,
                name: token.name,
                access_key_id: credential.access_key_id,
                prefix: credential.prefix,
                created_at: credential.created_at.to_string(),
            })
        })
        .collect();
    scoped.commit().await?;
    Ok(Json(tokens))
}

#[utoipa::path(get, operation_id = "storage_get_access_token", path = "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
    params(("organization_id" = Uuid, Path, description = "Organization ID"), ("project_id" = Uuid, Path, description = "Project ID"), ("token_id" = Uuid, Path, description = "Access token ID")),
    responses((status = 200, description = "Access token details", body = AccessTokenDetailsResponse), (status = 404, description = "Project or access token not found")), tag = "storage")]
pub async fn get_access_token(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id, token_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<AccessTokenDetailsResponse>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, project_id, organization_id).await?;
    let token = token_by_id(tx, organization_id, project_id, token_id).await?;
    let grants = bucket_grant::Entity::find()
        .filter(bucket_grant::Column::CredentialId.eq(token_id))
        .all(tx)
        .await?;
    let mut buckets = storage::Entity::find()
        .filter(
            storage::Column::BucketId.is_in(
                grants
                    .iter()
                    .map(|grant| grant.bucket_id)
                    .collect::<Vec<_>>(),
            ),
        )
        .all(tx)
        .await?;
    buckets.sort_by(|a, b| a.name.cmp(&b.name));
    let permissions = grants
        .iter()
        .map(|grant| (grant.bucket_id, (grant.can_read, grant.can_write)))
        .collect::<HashMap<_, _>>();
    let bucket_permissions = buckets
        .into_iter()
        .map(|bucket| {
            let (can_read, can_write) = permissions[&bucket.bucket_id];
            BucketPermissionRequest {
                bucket_id: bucket.id,
                can_read,
                can_write,
            }
        })
        .collect();
    scoped.commit().await?;
    Ok(Json(AccessTokenDetailsResponse {
        token,
        bucket_permissions,
    }))
}

#[utoipa::path(patch, operation_id = "storage_update_access_token", path = "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}", request_body = UpdateAccessTokenRequest,
    params(("organization_id" = Uuid, Path, description = "Organization ID"), ("project_id" = Uuid, Path, description = "Project ID"), ("token_id" = Uuid, Path, description = "Access token ID")),
    responses((status = 204, description = "Access token permissions updated"), (status = 404, description = "Project, access token, or bucket not found"), (status = 409, description = "Invalid permissions")), tag = "storage")]
pub async fn update_access_token(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, project_id, token_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<UpdateAccessTokenRequest>,
) -> Result<axum::http::StatusCode, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    verify_org_owner(&tenant_db, organization_id)?;
    if !valid_bucket_permissions(&body.bucket_permissions) {
        return Err(AppError::Conflict("Invalid bucket permissions".into()));
    }
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, project_id, organization_id).await?;
    let token = token_by_id(tx, organization_id, project_id, token_id).await?;
    let foundation_bucket_ids =
        verify_bucket_permissions(tx, organization_id, project_id, &body.bucket_permissions)
            .await?;
    bucket_grant::Entity::delete_many()
        .filter(bucket_grant::Column::CredentialId.eq(token_id))
        .exec(tx)
        .await?;
    insert_grants(
        tx,
        token_id,
        organization_id,
        &foundation_bucket_ids,
        &body.bucket_permissions,
    )
    .await?;
    let bucket_ids = body
        .bucket_permissions
        .iter()
        .map(|permission| permission.bucket_id)
        .collect::<Vec<_>>();
    crate::services::events::record(tx, organization_id, project_id, "storage-access-token:updated", serde_json::json!({ "summary": "Updated storage access token permissions", "target_id": token_id.to_string(), "bucket_ids": bucket_ids }), auth.actor_id).await?;
    scoped.commit().await?;
    if let Err(error) = get_app_state()
        .s3_providers
        .invalidate_access_token_cache(&token.access_key_id)
        .await
    {
        tracing::warn!(%error, %token_id, "access token cache invalidation failed after update");
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(delete, operation_id = "storage_revoke_access_token", path = "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
    params(("organization_id" = Uuid, Path, description = "Organization ID"), ("project_id" = Uuid, Path, description = "Project ID"), ("token_id" = Uuid, Path, description = "Access token ID")),
    responses((status = 200, description = "Access token revoked"), (status = 404, description = "Project or access token not found")), tag = "storage")]
pub async fn revoke_access_token(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, project_id, token_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    verify_org_owner(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, project_id, organization_id).await?;
    let token = token_by_id(tx, organization_id, project_id, token_id).await?;
    credential::ActiveModel {
        id: Set(token_id),
        revoked_at: Set(Some(Utc::now().into())),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    }
    .update(tx)
    .await?;
    crate::services::events::record(tx, organization_id, project_id, "storage-access-token:revoked", serde_json::json!({ "summary": format!("Revoked storage access token '{}'", token.name), "target_id": token_id.to_string() }), auth.actor_id).await?;
    scoped.commit().await?;
    if let Err(error) = get_app_state()
        .s3_providers
        .invalidate_access_token_cache(&token.access_key_id)
        .await
    {
        tracing::warn!(%error, %token_id, "access token cache invalidation failed after revoke");
    }
    Ok(Json(serde_json::json!({ "success": true })))
}

async fn verify_bucket_permissions(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
    permissions: &[BucketPermissionRequest],
) -> Result<Vec<Uuid>, AppError> {
    let bucket_ids = permissions
        .iter()
        .map(|permission| permission.bucket_id)
        .collect::<Vec<_>>();
    let buckets = storage::Entity::find()
        .filter(storage::Column::Id.is_in(bucket_ids))
        .filter(storage::Column::ProjectId.eq(project_id))
        .filter(storage::Column::OrganizationId.eq(organization_id))
        .all(tx)
        .await?
        .into_iter()
        .map(|bucket| (bucket.id, bucket.bucket_id))
        .collect::<HashMap<_, _>>();
    permissions
        .iter()
        .map(|permission| {
            buckets
                .get(&permission.bucket_id)
                .copied()
                .ok_or_else(|| AppError::NotFound("Bucket not found in this project".into()))
        })
        .collect()
}

async fn insert_grants(
    tx: &DatabaseTransaction,
    credential_id: Uuid,
    organization_id: Uuid,
    bucket_ids: &[Uuid],
    permissions: &[BucketPermissionRequest],
) -> Result<(), AppError> {
    let grants = bucket_ids
        .iter()
        .zip(permissions)
        .map(|(bucket_id, permission)| bucket_grant::ActiveModel {
            id: Set(Uuid::new_v4()),
            credential_id: Set(credential_id),
            bucket_id: Set(*bucket_id),
            organization_id: Set(Some(organization_id)),
            prefix: Set("".into()),
            can_read: Set(permission.can_read),
            can_write: Set(permission.can_write),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    if !grants.is_empty() {
        bucket_grant::Entity::insert_many(grants).exec(tx).await?;
    }
    Ok(())
}

async fn active_token_named(
    tx: &DatabaseTransaction,
    project_id: Uuid,
    name: &str,
) -> Result<bool, AppError> {
    let exists = storage_access_token::Entity::find()
        .filter(storage_access_token::Column::ProjectId.eq(project_id))
        .filter(storage_access_token::Column::Name.eq(name))
        .join(
            JoinType::InnerJoin,
            storage_access_token::Relation::Credential.def(),
        )
        .filter(credential::Column::RevokedAt.is_null())
        .one(tx)
        .await?
        .is_some();
    Ok(exists)
}

async fn token_by_id(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
    credential_id: Uuid,
) -> Result<AccessTokenResponse, AppError> {
    let (token, credential) = storage_access_token::Entity::find()
        .filter(storage_access_token::Column::CredentialId.eq(credential_id))
        .filter(storage_access_token::Column::OrganizationId.eq(organization_id))
        .filter(storage_access_token::Column::ProjectId.eq(project_id))
        .find_also_related(credential::Entity)
        .filter(credential::Column::RevokedAt.is_null())
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("S3 access token not found".into()))?;
    let credential =
        credential.ok_or_else(|| AppError::NotFound("S3 access token not found".into()))?;
    Ok(AccessTokenResponse {
        id: token.credential_id,
        name: token.name,
        access_key_id: credential.access_key_id,
        prefix: credential.prefix,
        created_at: credential.created_at.to_string(),
    })
}

fn valid_bucket_permissions(permissions: &[BucketPermissionRequest]) -> bool {
    !permissions.is_empty()
        && permissions
            .iter()
            .all(|permission| permission.can_read || permission.can_write)
        && permissions
            .iter()
            .map(|permission| permission.bucket_id)
            .collect::<HashSet<_>>()
            .len()
            == permissions.len()
}

fn valid_credential_prefix(prefix: &str) -> bool {
    prefix.len() <= MAX_CREDENTIAL_PREFIX_BYTES
}

#[cfg(test)]
mod tests {
    use super::{BucketPermissionRequest, valid_bucket_permissions, valid_credential_prefix};
    use uuid::Uuid;
    #[test]
    fn rejects_empty_and_duplicate_bucket_permissions() {
        assert!(!valid_bucket_permissions(&[]));
        let permission = BucketPermissionRequest {
            bucket_id: Uuid::nil(),
            can_read: true,
            can_write: true,
        };
        assert!(!valid_bucket_permissions(&[permission.clone(), permission]));
    }

    #[test]
    fn limits_credential_prefix_length() {
        assert!(valid_credential_prefix(&"p".repeat(1024)));
        assert!(!valid_credential_prefix(&"p".repeat(1025)));
    }
}
