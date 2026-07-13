use axum::{Json, extract::Path};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError,
    middleware::auth::AuthContext,
    models::entities::{bucket, storage_access_token, storage_access_token_bucket},
    state::get_app_state,
};

use super::databases::{verify_org_access, verify_project_in_org};

const ACCESS_KEY_PREFIX: &str = "CP";

#[derive(Deserialize, ToSchema)]
pub struct CreateAccessTokenRequest {
    pub name: String,
    pub bucket_permissions: Vec<BucketPermissionRequest>,
}

#[derive(Clone, Deserialize, ToSchema)]
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
    pub created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct CreatedAccessTokenResponse {
    #[serde(flatten)]
    pub token: AccessTokenResponse,
    pub secret_access_key: String,
    pub endpoint_url: String,
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens",
    request_body = CreateAccessTokenRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
    ),
    responses(
        (status = 201, description = "Access token created; save the secret access key now", body = CreatedAccessTokenResponse),
        (status = 404, description = "Project or bucket not found"),
        (status = 409, description = "Invalid permissions or duplicate token name"),
    ),
    tag = "storage",
)]
pub async fn create_access_token(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateAccessTokenRequest>,
) -> Result<(axum::http::StatusCode, Json<CreatedAccessTokenResponse>), AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let name = body.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::Conflict(
            "Token name must be 1-100 characters".into(),
        ));
    }
    if !valid_bucket_permissions(&body.bucket_permissions) {
        return Err(AppError::Conflict("Invalid bucket permissions".into()));
    }

    let state = get_app_state();
    let secrets = state.s3_providers.ok_or_else(|| {
        AppError::Internal("Control-plane secret service is not configured".into())
    })?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, project_id, organization_id).await?;
    for permission in &body.bucket_permissions {
        if bucket::Entity::find_by_id(permission.bucket_id)
            .filter(bucket::Column::ProjectId.eq(project_id))
            .filter(bucket::Column::OrganizationId.eq(organization_id))
            .one(tx)
            .await?
            .is_none()
        {
            return Err(AppError::NotFound(
                "Bucket not found in this project".into(),
            ));
        }
    }
    if storage_access_token::Entity::find()
        .filter(storage_access_token::Column::ProjectId.eq(project_id))
        .filter(storage_access_token::Column::Name.eq(name))
        .filter(storage_access_token::Column::RevokedAt.is_null())
        .one(tx)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(
            "A token with this name already exists".into(),
        ));
    }

    let id = Uuid::new_v4();
    let access_key_id = format!("{ACCESS_KEY_PREFIX}{}", Uuid::new_v4().simple()).to_uppercase();
    let secret_access_key = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    secrets
        .store_access_token_secret(id, &secret_access_key)
        .await?;
    let insert = async {
        let token = storage_access_token::ActiveModel {
            id: Set(id),
            organization_id: Set(organization_id),
            project_id: Set(project_id),
            name: Set(name.into()),
            access_key_id: Set(access_key_id),
            ..Default::default()
        }
        .insert(tx)
        .await?;
        for permission in &body.bucket_permissions {
            storage_access_token_bucket::ActiveModel {
                access_token_id: Set(id),
                bucket_id: Set(permission.bucket_id),
                organization_id: Set(organization_id),
                can_read: Set(permission.can_read),
                can_write: Set(permission.can_write),
            }
            .insert(tx)
            .await?;
        }
        Ok::<_, sea_orm::DbErr>(token)
    }
    .await;
    let token = match insert {
        Ok(token) => token,
        Err(error) => {
            let _ = secrets.delete_access_token_secret(id).await;
            return Err(error.into());
        }
    };
    if let Err(error) = scoped.commit().await {
        let _ = secrets.delete_access_token_secret(id).await;
        return Err(error);
    }

    Ok((
        axum::http::StatusCode::CREATED,
        Json(CreatedAccessTokenResponse {
            token: response(&token),
            secret_access_key,
            endpoint_url: state.config.storage_endpoint_url,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
    ),
    responses(
        (status = 200, description = "List of active access tokens", body = Vec<AccessTokenResponse>),
        (status = 404, description = "Project not found"),
    ),
    tag = "storage",
)]
pub async fn list_access_tokens(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<AccessTokenResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, project_id, organization_id).await?;
    let tokens = storage_access_token::Entity::find()
        .filter(storage_access_token::Column::ProjectId.eq(project_id))
        .filter(storage_access_token::Column::RevokedAt.is_null())
        .order_by_desc(storage_access_token::Column::CreatedAt)
        .all(tx)
        .await?;
    scoped.commit().await?;
    Ok(Json(tokens.iter().map(response).collect()))
}

#[utoipa::path(
    delete,
    path = "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("token_id" = Uuid, Path, description = "Access token ID"),
    ),
    responses(
        (status = 200, description = "Access token revoked"),
        (status = 404, description = "Project or access token not found"),
    ),
    tag = "storage",
)]
pub async fn revoke_access_token(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id, token_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, project_id, organization_id).await?;
    let token = storage_access_token::Entity::find_by_id(token_id)
        .filter(storage_access_token::Column::OrganizationId.eq(organization_id))
        .filter(storage_access_token::Column::ProjectId.eq(project_id))
        .filter(storage_access_token::Column::RevokedAt.is_null())
        .one(tx)
        .await?;
    let token = token.ok_or_else(|| AppError::NotFound("S3 access token not found".into()))?;
    let mut token = token.into_active_model();
    token.revoked_at = Set(Some(chrono::Utc::now().fixed_offset()));
    token.update(tx).await?;
    scoped.commit().await?;

    if let Some(secrets) = get_app_state().s3_providers
        && let Err(error) = secrets.delete_access_token_secret(token_id).await
    {
        tracing::warn!(%error, %token_id, "revoked S3 token secret cleanup failed");
    }
    Ok(Json(serde_json::json!({ "success": true })))
}

fn response(token: &storage_access_token::Model) -> AccessTokenResponse {
    AccessTokenResponse {
        id: token.id,
        name: token.name.clone(),
        access_key_id: token.access_key_id.clone(),
        created_at: token.created_at.to_rfc3339(),
    }
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

#[cfg(test)]
mod tests {
    use super::{BucketPermissionRequest, valid_bucket_permissions};
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
}
