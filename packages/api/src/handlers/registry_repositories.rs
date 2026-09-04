use axum::{Json, extract::Path, http::StatusCode};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError, middleware::auth::AuthContext, models::entities::registry_repository,
};
use lib::operation::{Operation, registry_repository_delete::RegistryRepositoryDelete};

use super::databases::{verify_org_access, verify_project_in_org};

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
    verify_org_access(&tenant_db, organization_id)?;
    let name = body.name.trim();
    if !valid_repository_name(name) {
        return Err(AppError::Conflict(
            "Repository names must use lowercase letters, numbers, dots, underscores, dashes, and slashes"
                .into(),
        ));
    }

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, project_id, organization_id).await?;
    super::managed_registry::require_active(tx, organization_id).await?;
    if registry_repository::Entity::find()
        .filter(registry_repository::Column::ProjectId.eq(project_id))
        .filter(registry_repository::Column::Name.eq(name))
        .one(tx)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict("Repository already exists".into()));
    }
    let created = registry_repository::ActiveModel {
        id: Set(Uuid::new_v4()),
        organization_id: Set(organization_id),
        project_id: Set(project_id),
        name: Set(name.into()),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    crate::services::events::record(
        tx,
        organization_id,
        project_id,
        "registry-repository:created",
        json!({ "summary": format!("Created registry repository '{name}'"), "target_id": created.id }),
        auth.actor_id,
    )
    .await?;
    scoped.commit().await?;

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
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, project_id, organization_id).await?;
    let repositories = registry_repository::Entity::find()
        .filter(registry_repository::Column::ProjectId.eq(project_id))
        .order_by_asc(registry_repository::Column::Name)
        .all(tx)
        .await?;
    scoped.commit().await?;
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
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, project_id, organization_id).await?;
    let repository = registry_repository::Entity::find_by_id(repository_id)
        .filter(registry_repository::Column::OrganizationId.eq(organization_id))
        .filter(registry_repository::Column::ProjectId.eq(project_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Registry repository not found".into()))?;

    Operation::<RegistryRepositoryDelete>::new(
        tx,
        organization_id,
        repository.project_id,
        repository.id,
    )
    .await?;
    crate::services::events::record(
        tx,
        organization_id,
        project_id,
        "registry-repository:deleted",
        json!({ "summary": format!("Deleted registry repository '{}'", repository.name), "target_id": repository.id }),
        auth.actor_id,
    )
    .await?;
    registry_repository::Entity::delete_by_id(repository.id)
        .exec(tx)
        .await?;
    scoped.commit().await?;

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

fn valid_repository_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 200
        && name.split('/').all(|segment| {
            let bytes = segment.as_bytes();
            !bytes.is_empty()
                && bytes.first().is_some_and(u8::is_ascii_alphanumeric)
                && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
                && bytes.iter().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'.' | b'_' | b'-')
                })
                && !bytes.windows(2).any(|pair| {
                    !pair[0].is_ascii_alphanumeric() && !pair[1].is_ascii_alphanumeric()
                })
        })
}

#[cfg(test)]
mod tests {
    use super::valid_repository_name;

    #[test]
    fn validates_distribution_repository_names() {
        assert!(valid_repository_name("backend/api-v2"));
        assert!(!valid_repository_name("Backend"));
        assert!(!valid_repository_name("backend//api"));
        assert!(!valid_repository_name("backend..api"));
    }
}
