use axum::{Json, extract::Path, http::StatusCode};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{errors::AppError, middleware::auth::AuthContext, state::get_app_state};
use lib::services::registry_tags;

#[derive(Serialize, ToSchema)]
pub struct RepositoryTagsResponse {
    pub tags: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/projects/{project_id}/registry/repositories/{repository_id}/tags",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("repository_id" = Uuid, Path, description = "Registry repository ID"),
    ),
    responses(
        (status = 200, description = "Repository tag names", body = RepositoryTagsResponse),
        (status = 403, description = "Organization access required"),
        (status = 404, description = "Project or registry repository not found"),
        (status = 503, description = "Container registry is unavailable"),
    ),
    tag = "registry",
)]
pub async fn list_tags(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id, repository_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<RepositoryTagsResponse>, AppError> {
    let state = get_app_state();
    let context = registry_tags::Context {
        identity_db: state.identity_db.connection(),
        client: &state.storage_client,
        token_ttl_seconds: state.config.registry_token_ttl_seconds,
    };
    let tags = registry_tags::list(
        &tenant_db,
        &context,
        organization_id,
        project_id,
        repository_id,
    )
    .await?;
    Ok(Json(RepositoryTagsResponse { tags }))
}

#[utoipa::path(
    delete,
    path = "/api/organization/{organization_id}/projects/{project_id}/registry/repositories/{repository_id}/tags/{tag}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("repository_id" = Uuid, Path, description = "Registry repository ID"),
        ("tag" = String, Path, description = "Tag name"),
    ),
    responses(
        (status = 204, description = "Tag deleted"),
        (status = 403, description = "Organization access required"),
        (status = 404, description = "Project, registry repository, or tag not found"),
        (status = 503, description = "Container registry is unavailable"),
    ),
    tag = "registry",
)]
pub async fn delete_tag(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, project_id, repository_id, tag)): Path<(Uuid, Uuid, Uuid, String)>,
) -> Result<StatusCode, AppError> {
    let state = get_app_state();
    let context = registry_tags::Context {
        identity_db: state.identity_db.connection(),
        client: &state.storage_client,
        token_ttl_seconds: state.config.registry_token_ttl_seconds,
    };
    registry_tags::delete(
        &tenant_db,
        &context,
        organization_id,
        project_id,
        repository_id,
        &tag,
        auth.actor_id,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
