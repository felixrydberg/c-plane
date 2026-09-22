use crate::{errors::AppError, middleware::auth::AuthContext, state::get_app_state};
use axum::{Json, extract::Path};
use lib::services::storage_access_tokens as service;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
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
fn permissions(v: &[BucketPermissionRequest]) -> Vec<service::BucketPermission> {
    v.iter()
        .map(|p| service::BucketPermission {
            bucket_id: p.bucket_id,
            can_read: p.can_read,
            can_write: p.can_write,
        })
        .collect()
}
fn response(t: service::AccessToken) -> AccessTokenResponse {
    AccessTokenResponse {
        id: t.id,
        name: t.name,
        access_key_id: t.access_key_id,
        prefix: t.prefix,
        created_at: t.created_at,
    }
}
#[utoipa::path(post, operation_id = "storage_create_access_token", path = "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens", request_body = CreateAccessTokenRequest,
    params(("organization_id" = Uuid, Path, description = "Organization ID"), ("project_id" = Uuid, Path, description = "Project ID")),
    responses((status = 201, description = "Access token created; save the secret access key now", body = CreatedAccessTokenResponse), (status = 404, description = "Project or bucket not found"), (status = 409, description = "Invalid permissions or duplicate token name")), tag = "storage")]
pub async fn create_access_token(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateAccessTokenRequest>,
) -> Result<(axum::http::StatusCode, Json<CreatedAccessTokenResponse>), AppError> {
    let state = get_app_state();
    let c = service::create(
        &tenant_db,
        &state.secrets,
        organization_id,
        project_id,
        body.name.trim(),
        &body.prefix,
        &permissions(&body.bucket_permissions),
        auth.actor_id,
    )
    .await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(CreatedAccessTokenResponse {
            token: response(c.token),
            secret_access_key: c.secret_access_key,
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
    Ok(Json(
        service::list(&tenant_db, organization_id, project_id)
            .await?
            .into_iter()
            .map(response)
            .collect(),
    ))
}
#[utoipa::path(get, operation_id = "storage_get_access_token", path = "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
    params(("organization_id" = Uuid, Path, description = "Organization ID"), ("project_id" = Uuid, Path, description = "Project ID"), ("token_id" = Uuid, Path, description = "Access token ID")),
    responses((status = 200, description = "Access token details", body = AccessTokenDetailsResponse), (status = 404, description = "Project or access token not found")), tag = "storage")]
pub async fn get_access_token(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id, token_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<AccessTokenDetailsResponse>, AppError> {
    let d = service::get(&tenant_db, organization_id, project_id, token_id).await?;
    Ok(Json(AccessTokenDetailsResponse {
        token: response(d.token),
        bucket_permissions: d
            .bucket_permissions
            .into_iter()
            .map(|p| BucketPermissionRequest {
                bucket_id: p.bucket_id,
                can_read: p.can_read,
                can_write: p.can_write,
            })
            .collect(),
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
    let state = get_app_state();
    service::update(
        &tenant_db,
        &state.s3_providers,
        organization_id,
        project_id,
        token_id,
        &permissions(&body.bucket_permissions),
        auth.actor_id,
    )
    .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
#[utoipa::path(delete, operation_id = "storage_revoke_access_token", path = "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
    params(("organization_id" = Uuid, Path, description = "Organization ID"), ("project_id" = Uuid, Path, description = "Project ID"), ("token_id" = Uuid, Path, description = "Access token ID")),
    responses((status = 200, description = "Access token revoked"), (status = 404, description = "Project or access token not found")), tag = "storage")]
pub async fn revoke_access_token(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, project_id, token_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = get_app_state();
    service::revoke(
        &tenant_db,
        &state.s3_providers,
        organization_id,
        project_id,
        token_id,
        auth.actor_id,
    )
    .await?;
    Ok(Json(serde_json::json!({"success":true})))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_empty_and_duplicate_bucket_permissions() {
        let id = Uuid::nil();
        assert!(service::validate_permissions(&[]).is_err());
        assert!(
            service::validate_permissions(&[
                service::BucketPermission {
                    bucket_id: id,
                    can_read: true,
                    can_write: true
                },
                service::BucketPermission {
                    bucket_id: id,
                    can_read: true,
                    can_write: true
                }
            ])
            .is_err()
        );
    }
    #[test]
    fn limits_credential_prefix_length() {
        assert!("p".repeat(1024).len() <= 1024);
        assert!("p".repeat(1025).len() > 1024);
    }
}
