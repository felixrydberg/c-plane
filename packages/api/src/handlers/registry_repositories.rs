use axum::{Json, extract::Path, http::StatusCode};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError, middleware::auth::AuthContext, models::entities::registry_repository,
};
use lib::services::registry_repositories as repository_service;

#[derive(Deserialize, ToSchema)]
pub struct CreateRegistryRepositoryRequest {
    pub name: String,
}

#[derive(Serialize, ToSchema)]
pub struct RegistryRepositoryResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub created_at: String,
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/projects/{project_id}/registry/repositories",
    request_body = CreateRegistryRepositoryRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
    ),
    responses(
        (status = 201, description = "Registry repository created", body = RegistryRepositoryResponse),
        (status = 403, description = "Organization access required"),
        (status = 404, description = "Project not found"),
        (status = 409, description = "Invalid or duplicate repository name"),
    ),
    tag = "registry",
)]
pub async fn create_repository(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateRegistryRepositoryRequest>,
) -> Result<(StatusCode, Json<RegistryRepositoryResponse>), AppError> {
    let created = repository_service::create(
        &tenant_db,
        organization_id,
        auth.actor_id,
        repository_service::CreateInput {
            project_id,
            name: body.name,
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(response(&created))))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/projects/{project_id}/registry/repositories",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
    ),
    responses(
        (status = 200, description = "Project registry repositories", body = Vec<RegistryRepositoryResponse>),
        (status = 403, description = "Organization access required"),
        (status = 404, description = "Project not found"),
    ),
    tag = "registry",
)]
pub async fn list_repositories(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<RegistryRepositoryResponse>>, AppError> {
    let repositories = repository_service::list(&tenant_db, organization_id, project_id).await?;
    Ok(Json(repositories.iter().map(response).collect()))
}

#[utoipa::path(
    delete,
    path = "/api/organization/{organization_id}/projects/{project_id}/registry/repositories/{repository_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("repository_id" = Uuid, Path, description = "Registry repository ID"),
    ),
    responses(
        (status = 202, description = "Registry repository deletion queued"),
        (status = 403, description = "Organization access required"),
        (status = 404, description = "Project or registry repository not found"),
    ),
    tag = "registry",
)]
pub async fn delete_repository(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, project_id, repository_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    repository_service::delete(
        &tenant_db,
        organization_id,
        auth.actor_id,
        project_id,
        repository_id,
    )
    .await?;

    Ok(StatusCode::ACCEPTED)
}

fn response(repository: &registry_repository::Model) -> RegistryRepositoryResponse {
    RegistryRepositoryResponse {
        id: repository.id,
        project_id: repository.project_id,
        name: repository.name.clone(),
        created_at: repository.created_at.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use lib::services::registry_repositories::valid_name;

    #[test]
    fn validates_distribution_repository_names() {
        assert!(valid_name("backend/api-v2"));
        assert!(!valid_name("Backend"));
        assert!(!valid_name("backend//api"));
        assert!(!valid_name("backend..api"));
    }
}
