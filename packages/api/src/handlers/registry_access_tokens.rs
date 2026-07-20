use axum::{Json, extract::Path, http::StatusCode};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError,
    middleware::auth::AuthContext,
    models::entities::{
        event, registry_access_token, registry_repository, registry_repository_grant,
    },
};

use super::databases::verify_org_access;

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct RepositoryPermissionRequest {
    pub repository_id: Uuid,
    pub can_pull: bool,
    pub can_push: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateRegistryAccessTokenRequest {
    pub name: String,
    pub repository_permissions: Vec<RepositoryPermissionRequest>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateRegistryAccessTokenRequest {
    pub repository_permissions: Vec<RepositoryPermissionRequest>,
}

#[derive(Serialize, ToSchema)]
pub struct RegistryAccessTokenResponse {
    pub id: Uuid,
    pub name: String,
    pub created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct CreatedRegistryAccessTokenResponse {
    #[serde(flatten)]
    pub access_token: RegistryAccessTokenResponse,
    pub token: String,
}

#[derive(Serialize, ToSchema)]
pub struct RegistryAccessTokenDetailsResponse {
    #[serde(flatten)]
    pub access_token: RegistryAccessTokenResponse,
    pub repository_permissions: Vec<RepositoryPermissionRequest>,
}

#[utoipa::path(
    post,
    operation_id = "registry_create_access_token",
    path = "/api/organization/{organization_id}/registry/access-tokens",
    request_body = CreateRegistryAccessTokenRequest,
    params(("organization_id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 201, description = "Registry access token created; save it now", body = CreatedRegistryAccessTokenResponse),
        (status = 403, description = "Organization access required"),
        (status = 404, description = "Repository not found"),
        (status = 409, description = "Invalid permissions or duplicate token name"),
    ),
    tag = "registry",
)]
pub async fn create_access_token(
    AuthContext { tenant_db, auth }: AuthContext,
    Path(organization_id): Path<Uuid>,
    Json(body): Json<CreateRegistryAccessTokenRequest>,
) -> Result<(StatusCode, Json<CreatedRegistryAccessTokenResponse>), AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let name = body.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::Conflict(
            "Token name must be 1-100 characters".into(),
        ));
    }
    if !valid_permissions(&body.repository_permissions) {
        return Err(AppError::Conflict(
            "Select at least one valid repository permission".into(),
        ));
    }

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_repositories(tx, organization_id, &body.repository_permissions).await?;
    if registry_access_token::Entity::find()
        .filter(registry_access_token::Column::OrganizationId.eq(organization_id))
        .filter(registry_access_token::Column::Name.eq(name))
        .filter(registry_access_token::Column::RevokedAt.is_null())
        .one(tx)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(
            "An active token with this name already exists".into(),
        ));
    }

    let id = Uuid::new_v4();
    let token = generate_token();
    let created = registry_access_token::ActiveModel {
        id: Set(id),
        organization_id: Set(organization_id),
        name: Set(name.into()),
        token_hash: Set(hex::encode(Sha256::digest(token.as_bytes()))),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    replace_permissions(tx, organization_id, id, &body.repository_permissions).await?;
    record_event(
        tx,
        organization_id,
        auth.actor_id,
        "registry-access-token:created",
        json!({
            "summary": format!("Created registry access token '{name}'"),
            "target_id": id,
            "repository_ids": body.repository_permissions.iter().map(|permission| permission.repository_id).collect::<Vec<_>>(),
        }),
    )
    .await?;
    scoped.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(CreatedRegistryAccessTokenResponse {
            access_token: response(&created),
            token,
        }),
    ))
}

#[utoipa::path(
    get,
    operation_id = "registry_list_access_tokens",
    path = "/api/organization/{organization_id}/registry/access-tokens",
    params(("organization_id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "List of active registry access tokens", body = Vec<RegistryAccessTokenResponse>),
        (status = 403, description = "Organization access required"),
    ),
    tag = "registry",
)]
pub async fn list_access_tokens(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
) -> Result<Json<Vec<RegistryAccessTokenResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let tokens = registry_access_token::Entity::find()
        .filter(registry_access_token::Column::OrganizationId.eq(organization_id))
        .filter(registry_access_token::Column::RevokedAt.is_null())
        .order_by_desc(registry_access_token::Column::CreatedAt)
        .all(tx)
        .await?;
    scoped.commit().await?;
    Ok(Json(tokens.iter().map(response).collect()))
}

#[utoipa::path(
    get,
    operation_id = "registry_get_access_token",
    path = "/api/organization/{organization_id}/registry/access-tokens/{token_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("token_id" = Uuid, Path, description = "Registry access token ID"),
    ),
    responses(
        (status = 200, description = "Registry access token details", body = RegistryAccessTokenDetailsResponse),
        (status = 404, description = "Registry access token not found"),
    ),
    tag = "registry",
)]
pub async fn get_access_token(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, token_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RegistryAccessTokenDetailsResponse>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let token = active_token(tx, organization_id, token_id).await?;
    let repository_permissions = registry_repository_grant::Entity::find()
        .filter(registry_repository_grant::Column::AccessTokenId.eq(token_id))
        .all(tx)
        .await?
        .into_iter()
        .map(|permission| RepositoryPermissionRequest {
            repository_id: permission.repository_id,
            can_pull: permission.can_pull,
            can_push: permission.can_push,
        })
        .collect();
    scoped.commit().await?;
    Ok(Json(RegistryAccessTokenDetailsResponse {
        access_token: response(&token),
        repository_permissions,
    }))
}

#[utoipa::path(
    patch,
    operation_id = "registry_update_access_token",
    path = "/api/organization/{organization_id}/registry/access-tokens/{token_id}",
    request_body = UpdateRegistryAccessTokenRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("token_id" = Uuid, Path, description = "Registry access token ID"),
    ),
    responses(
        (status = 204, description = "Registry access token permissions updated"),
        (status = 403, description = "Organization access required"),
        (status = 404, description = "Registry access token or repository not found"),
        (status = 409, description = "Invalid permissions"),
    ),
    tag = "registry",
)]
pub async fn update_access_token(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, token_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateRegistryAccessTokenRequest>,
) -> Result<StatusCode, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    if !valid_permissions(&body.repository_permissions) {
        return Err(AppError::Conflict(
            "Select at least one valid repository permission".into(),
        ));
    }

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    active_token(tx, organization_id, token_id).await?;
    verify_repositories(tx, organization_id, &body.repository_permissions).await?;
    replace_permissions(tx, organization_id, token_id, &body.repository_permissions).await?;
    record_event(
        tx,
        organization_id,
        auth.actor_id,
        "registry-access-token:updated",
        json!({
            "summary": "Updated registry access token permissions",
            "target_id": token_id,
            "repository_ids": body.repository_permissions.iter().map(|permission| permission.repository_id).collect::<Vec<_>>(),
        }),
    )
    .await?;
    scoped.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    operation_id = "registry_revoke_access_token",
    path = "/api/organization/{organization_id}/registry/access-tokens/{token_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("token_id" = Uuid, Path, description = "Registry access token ID"),
    ),
    responses(
        (status = 204, description = "Registry access token revoked"),
        (status = 403, description = "Organization access required"),
        (status = 404, description = "Registry access token not found"),
    ),
    tag = "registry",
)]
pub async fn revoke_access_token(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, token_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let token = active_token(tx, organization_id, token_id).await?;
    let name = token.name.clone();
    let mut token = token.into_active_model();
    token.revoked_at = Set(Some(Utc::now().fixed_offset()));
    token.update(tx).await?;
    record_event(
        tx,
        organization_id,
        auth.actor_id,
        "registry-access-token:revoked",
        json!({ "summary": format!("Revoked registry access token '{name}'"), "target_id": token_id }),
    )
    .await?;
    scoped.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

fn response(token: &registry_access_token::Model) -> RegistryAccessTokenResponse {
    RegistryAccessTokenResponse {
        id: token.id,
        name: token.name.clone(),
        created_at: token.created_at.to_rfc3339(),
    }
}

async fn active_token(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    token_id: Uuid,
) -> Result<registry_access_token::Model, AppError> {
    registry_access_token::Entity::find_by_id(token_id)
        .filter(registry_access_token::Column::OrganizationId.eq(organization_id))
        .filter(registry_access_token::Column::RevokedAt.is_null())
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Registry access token not found".into()))
}

async fn verify_repositories(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    permissions: &[RepositoryPermissionRequest],
) -> Result<(), AppError> {
    for permission in permissions {
        if registry_repository::Entity::find_by_id(permission.repository_id)
            .filter(registry_repository::Column::OrganizationId.eq(organization_id))
            .one(tx)
            .await?
            .is_none()
        {
            return Err(AppError::NotFound(
                "Repository not found in this organization".into(),
            ));
        }
    }
    Ok(())
}

async fn replace_permissions(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    token_id: Uuid,
    permissions: &[RepositoryPermissionRequest],
) -> Result<(), AppError> {
    registry_repository_grant::Entity::delete_many()
        .filter(registry_repository_grant::Column::AccessTokenId.eq(token_id))
        .exec(tx)
        .await?;
    for permission in permissions {
        registry_repository_grant::ActiveModel {
            id: Set(Uuid::new_v4()),
            organization_id: Set(organization_id),
            repository_id: Set(permission.repository_id),
            access_token_id: Set(token_id),
            can_pull: Set(permission.can_pull || permission.can_push),
            can_push: Set(permission.can_push),
            ..Default::default()
        }
        .insert(tx)
        .await?;
    }
    Ok(())
}

pub(super) async fn record_event(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    actor_id: Uuid,
    event_type: &str,
    payload: serde_json::Value,
) -> Result<(), AppError> {
    event::ActiveModel {
        id: Set(Uuid::new_v4()),
        organization_id: Set(organization_id),
        event_type: Set(event_type.into()),
        payload: Set(payload),
        system: Set(false),
        project_id: Set(None),
        actor_id: Set(Some(actor_id)),
        created_at: Set(Utc::now().naive_utc()),
    }
    .insert(tx)
    .await?;
    Ok(())
}

fn generate_token() -> String {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(Uuid::new_v4().as_bytes());
    bytes.extend_from_slice(Uuid::new_v4().as_bytes());
    format!("cr_{}", URL_SAFE_NO_PAD.encode(bytes))
}

fn valid_permissions(permissions: &[RepositoryPermissionRequest]) -> bool {
    !permissions.is_empty()
        && permissions
            .iter()
            .all(|permission| permission.can_pull || permission.can_push)
        && permissions
            .iter()
            .map(|permission| permission.repository_id)
            .collect::<HashSet<_>>()
            .len()
            == permissions.len()
}

#[cfg(test)]
mod tests {
    use super::{RepositoryPermissionRequest, valid_permissions};
    use uuid::Uuid;

    #[test]
    fn rejects_empty_and_duplicate_repository_permissions() {
        assert!(!valid_permissions(&[]));
        let permission = RepositoryPermissionRequest {
            repository_id: Uuid::nil(),
            can_pull: true,
            can_push: false,
        };
        assert!(!valid_permissions(&[permission.clone(), permission]));
    }
}
