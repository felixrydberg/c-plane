use axum::{Json, extract::Path, http::StatusCode};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError, middleware::auth::AuthContext, models::entities::registry_access_token,
};
use lib::services::registry_access_tokens as registry_service;

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct RepositoryPermissionRequest {
    pub repository_id: Uuid,
    pub can_pull: bool,
    pub can_push: bool,
}

impl From<RepositoryPermissionRequest> for registry_service::Permission {
    fn from(permission: RepositoryPermissionRequest) -> Self {
        Self {
            repository_id: permission.repository_id,
            can_pull: permission.can_pull,
            can_push: permission.can_push,
        }
    }
}

impl From<registry_service::Permission> for RepositoryPermissionRequest {
    fn from(permission: registry_service::Permission) -> Self {
        Self {
            repository_id: permission.repository_id,
            can_pull: permission.can_pull,
            can_push: permission.can_push,
        }
    }
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
    pub project_id: Uuid,
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
    path = "/api/organization/{organization_id}/projects/{project_id}/registry/access-tokens",
    request_body = CreateRegistryAccessTokenRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
    ),
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
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateRegistryAccessTokenRequest>,
) -> Result<(StatusCode, Json<CreatedRegistryAccessTokenResponse>), AppError> {
    let permissions = body
        .repository_permissions
        .into_iter()
        .map(Into::into)
        .collect::<Vec<_>>();
    let created = registry_service::create(
        &tenant_db,
        organization_id,
        project_id,
        auth.actor_id,
        body.name,
        &permissions,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedRegistryAccessTokenResponse {
            access_token: response(&created.token),
            token: created.plaintext,
        }),
    ))
}

#[utoipa::path(
    get,
    operation_id = "registry_list_access_tokens",
    path = "/api/organization/{organization_id}/projects/{project_id}/registry/access-tokens",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
    ),
    responses(
        (status = 200, description = "List of active registry access tokens", body = Vec<RegistryAccessTokenResponse>),
        (status = 403, description = "Organization access required"),
    ),
    tag = "registry",
)]
pub async fn list_access_tokens(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<RegistryAccessTokenResponse>>, AppError> {
    let tokens = registry_service::list(&tenant_db, organization_id, project_id).await?;
    Ok(Json(tokens.iter().map(response).collect()))
}

#[utoipa::path(
    get,
    operation_id = "registry_get_access_token",
    path = "/api/organization/{organization_id}/projects/{project_id}/registry/access-tokens/{token_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
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
    Path((organization_id, project_id, token_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<RegistryAccessTokenDetailsResponse>, AppError> {
    let details = registry_service::get(&tenant_db, organization_id, project_id, token_id).await?;
    Ok(Json(RegistryAccessTokenDetailsResponse {
        access_token: response(&details.token),
        repository_permissions: details.permissions.into_iter().map(Into::into).collect(),
    }))
}

#[utoipa::path(
    patch,
    operation_id = "registry_update_access_token",
    path = "/api/organization/{organization_id}/projects/{project_id}/registry/access-tokens/{token_id}",
    request_body = UpdateRegistryAccessTokenRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
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
    Path((organization_id, project_id, token_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<UpdateRegistryAccessTokenRequest>,
) -> Result<StatusCode, AppError> {
    let permissions = body
        .repository_permissions
        .into_iter()
        .map(Into::into)
        .collect::<Vec<_>>();
    registry_service::update(
        &tenant_db,
        organization_id,
        project_id,
        auth.actor_id,
        token_id,
        &permissions,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    operation_id = "registry_revoke_access_token",
    path = "/api/organization/{organization_id}/projects/{project_id}/registry/access-tokens/{token_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
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
    Path((organization_id, project_id, token_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    registry_service::revoke(
        &tenant_db,
        organization_id,
        project_id,
        auth.actor_id,
        token_id,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

fn response(token: &registry_access_token::Model) -> RegistryAccessTokenResponse {
    RegistryAccessTokenResponse {
        id: token.id,
        project_id: token.project_id,
        name: token.name.clone(),
        created_at: token.created_at.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use lib::services::registry_access_tokens::{Permission, valid_permissions};
    use uuid::Uuid;

    #[test]
    fn rejects_empty_and_duplicate_repository_permissions() {
        assert!(!valid_permissions(&[]));
        let permission = Permission {
            repository_id: Uuid::nil(),
            can_pull: true,
            can_push: false,
        };
        assert!(!valid_permissions(&[permission.clone(), permission]));
    }
}
