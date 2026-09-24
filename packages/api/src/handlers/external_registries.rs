use crate::{errors::AppError, middleware::auth::AuthContext, models::entities::external_registry};
use axum::{Json, extract::Path, http::StatusCode};
use lib::services::external_registries as service;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExternalRegistryProvider {
    DockerHub,
    Github,
    Gitlab,
    GoogleArtifactRegistry,
    AwsEcr,
}
#[derive(Deserialize, ToSchema)]
pub struct CreateExternalRegistryRequest {
    pub name: String,
    pub provider: ExternalRegistryProvider,
    pub host: Option<String>,
    pub username: String,
    pub token: String,
}
#[derive(Deserialize, ToSchema)]
pub struct RenameExternalRegistryRequest {
    pub name: String,
}
#[derive(Deserialize, ToSchema)]
pub struct RotateExternalRegistryTokenRequest {
    pub token: String,
}
#[derive(Serialize, ToSchema)]
pub struct ExternalRegistryResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub host: String,
    pub username: String,
    pub created_at: String,
    pub updated_at: String,
}

#[utoipa::path(get, path = "/api/organization/{organization_id}/registry/external-registries", params(("organization_id" = Uuid, Path, description = "Organization ID")), responses((status = 200, body = Vec<ExternalRegistryResponse>)), tag = "registry")]
pub async fn list_external_registries(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
) -> Result<Json<Vec<ExternalRegistryResponse>>, AppError> {
    Ok(Json(
        service::list(&tenant_db, organization_id)
            .await?
            .iter()
            .map(response)
            .collect(),
    ))
}
#[utoipa::path(post, path = "/api/organization/{organization_id}/registry/external-registries", request_body = CreateExternalRegistryRequest, params(("organization_id" = Uuid, Path, description = "Organization ID")), responses((status = 201, body = ExternalRegistryResponse), (status = 400, body = crate::errors::ErrorResponse), (status = 409, body = crate::errors::ErrorResponse)), tag = "registry")]
pub async fn create_external_registry(
    AuthContext { tenant_db, auth }: AuthContext,
    Path(organization_id): Path<Uuid>,
    Json(body): Json<CreateExternalRegistryRequest>,
) -> Result<(StatusCode, Json<ExternalRegistryResponse>), AppError> {
    let created = service::create(
        &tenant_db,
        &crate::state::get_app_state().secrets,
        organization_id,
        auth.actor_id,
        body.name,
        provider(body.provider),
        body.host.as_deref(),
        body.username,
        body.token,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(response(&created))))
}
#[utoipa::path(patch, path = "/api/organization/{organization_id}/registry/external-registries/{registry_id}", request_body = RenameExternalRegistryRequest, params(("organization_id" = Uuid, Path), ("registry_id" = Uuid, Path)), responses((status = 200, body = ExternalRegistryResponse), (status = 400, body = crate::errors::ErrorResponse), (status = 404), (status = 409, body = crate::errors::ErrorResponse)), tag = "registry")]
pub async fn rename_external_registry(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, registry_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<RenameExternalRegistryRequest>,
) -> Result<Json<ExternalRegistryResponse>, AppError> {
    Ok(Json(response(
        &service::rename(
            &tenant_db,
            organization_id,
            auth.actor_id,
            registry_id,
            body.name,
        )
        .await?,
    )))
}
#[utoipa::path(post, path = "/api/organization/{organization_id}/registry/external-registries/{registry_id}/rotate-token", request_body = RotateExternalRegistryTokenRequest, params(("organization_id" = Uuid, Path), ("registry_id" = Uuid, Path)), responses((status = 204), (status = 400, body = crate::errors::ErrorResponse), (status = 404), (status = 409, body = crate::errors::ErrorResponse)), tag = "registry")]
pub async fn rotate_external_registry_token(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, registry_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<RotateExternalRegistryTokenRequest>,
) -> Result<StatusCode, AppError> {
    service::rotate(
        &tenant_db,
        &crate::state::get_app_state().secrets,
        organization_id,
        auth.actor_id,
        registry_id,
        body.token,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(delete, path = "/api/organization/{organization_id}/registry/external-registries/{registry_id}", params(("organization_id" = Uuid, Path), ("registry_id" = Uuid, Path)), responses((status = 204), (status = 404), (status = 409, body = crate::errors::ErrorResponse)), tag = "registry")]
pub async fn delete_external_registry(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, registry_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    service::delete(&tenant_db, organization_id, auth.actor_id, registry_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
fn provider(value: ExternalRegistryProvider) -> service::Provider {
    match value {
        ExternalRegistryProvider::DockerHub => service::Provider::DockerHub,
        ExternalRegistryProvider::Github => service::Provider::Github,
        ExternalRegistryProvider::Gitlab => service::Provider::Gitlab,
        ExternalRegistryProvider::GoogleArtifactRegistry => {
            service::Provider::GoogleArtifactRegistry
        }
        ExternalRegistryProvider::AwsEcr => service::Provider::AwsEcr,
    }
}
fn response(registry: &external_registry::Model) -> ExternalRegistryResponse {
    ExternalRegistryResponse {
        id: registry.id,
        organization_id: registry.organization_id,
        name: registry.name.clone(),
        host: registry.host.clone(),
        username: registry.username.clone(),
        created_at: registry.created_at.to_rfc3339(),
        updated_at: registry.updated_at.to_rfc3339(),
    }
}
