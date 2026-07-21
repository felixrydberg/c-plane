use std::time::Duration;

use axum::{Json, extract::Path, http::StatusCode};
use reqwest::{Client, Url};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError, middleware::auth::AuthContext, models::entities::registry_repository,
};

use super::{
    databases::verify_org_access, registry::sign_repository_token,
    registry_access_tokens::record_event,
};

const MANIFEST_ACCEPT: &str = "application/vnd.oci.image.index.v1+json, application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.docker.distribution.manifest.v2+json";

#[derive(Deserialize)]
struct TagsResponse {
    tags: Option<Vec<String>>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateRegistryRepositoryRequest {
    pub name: String,
}

#[derive(Serialize, ToSchema)]
pub struct RegistryRepositoryResponse {
    pub id: Uuid,
    pub name: String,
    pub created_at: String,
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/registry/repositories",
    request_body = CreateRegistryRepositoryRequest,
    params(("organization_id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 201, description = "Registry repository created", body = RegistryRepositoryResponse),
        (status = 403, description = "Organization access required"),
        (status = 409, description = "Invalid or duplicate repository name"),
    ),
    tag = "registry",
)]
pub async fn create_repository(
    AuthContext { tenant_db, auth }: AuthContext,
    Path(organization_id): Path<Uuid>,
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
    if registry_repository::Entity::find()
        .filter(registry_repository::Column::OrganizationId.eq(organization_id))
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
        name: Set(name.into()),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    record_event(
        tx,
        organization_id,
        auth.actor_id,
        "registry-repository:created",
        json!({ "summary": format!("Created registry repository '{name}'"), "target_id": created.id }),
    )
    .await?;
    scoped.commit().await?;

    Ok((StatusCode::CREATED, Json(response(&created))))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/registry/repositories",
    params(("organization_id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Organization registry repositories", body = Vec<RegistryRepositoryResponse>),
        (status = 403, description = "Organization access required"),
    ),
    tag = "registry",
)]
pub async fn list_repositories(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
) -> Result<Json<Vec<RegistryRepositoryResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let repositories = registry_repository::Entity::find()
        .filter(registry_repository::Column::OrganizationId.eq(organization_id))
        .order_by_asc(registry_repository::Column::Name)
        .all(tx)
        .await?;
    scoped.commit().await?;
    Ok(Json(repositories.iter().map(response).collect()))
}

#[utoipa::path(
    delete,
    path = "/api/organization/{organization_id}/registry/repositories/{repository_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("repository_id" = Uuid, Path, description = "Registry repository ID"),
    ),
    responses(
        (status = 204, description = "Registry repository deleted"),
        (status = 403, description = "Organization access required"),
        (status = 404, description = "Registry repository not found"),
        (status = 503, description = "Registry is read-only for maintenance"),
        (status = 409, description = "Registry cleanup conflict"),
        (status = 500, description = "Registry cleanup failed"),
    ),
    tag = "registry",
)]
pub async fn delete_repository(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, repository_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let repository = registry_repository::Entity::find_by_id(repository_id)
        .filter(registry_repository::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Registry repository not found".into()))?;
    scoped.commit().await?;

    delete_repository_images(&repository.name, organization_id).await?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let repository = registry_repository::Entity::find_by_id(repository_id)
        .filter(registry_repository::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Registry repository not found".into()))?;

    registry_repository::Entity::delete_by_id(repository.id)
        .exec(tx)
        .await?;
    record_event(
        tx,
        organization_id,
        auth.actor_id,
        "registry-repository:deleted",
        json!({ "summary": format!("Deleted registry repository '{}'", repository.name), "target_id": repository.id }),
    )
    .await?;
    scoped.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_repository_images(
    repository_name: &str,
    organization_id: Uuid,
) -> Result<(), AppError> {
    let base_url =
        std::env::var("REGISTRY_INTERNAL_URL").unwrap_or_else(|_| "http://registry:5000".into());
    let token =
        sign_repository_token(organization_id, repository_name, &["pull", "delete"]).await?;
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| {
            AppError::Internal(format!("Registry cleanup client setup failed: {error}"))
        })?;
    let tags_url = registry_url(&base_url, repository_name, "tags/list")?;
    let response = client
        .get(tags_url)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|error| AppError::Internal(format!("Registry cleanup request failed: {error}")))?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(());
    }
    if !response.status().is_success() {
        return Err(AppError::Conflict(format!(
            "Registry cleanup failed while listing tags: {}",
            response.status()
        )));
    }
    let tags = response
        .json::<TagsResponse>()
        .await
        .map_err(|error| AppError::Internal(format!("Invalid registry tag response: {error}")))?
        .tags
        .unwrap_or_default();
    let mut digests = std::collections::HashSet::new();
    for tag in tags {
        let manifest_url = registry_url(&base_url, repository_name, &format!("manifests/{tag}"))?;
        let response = client
            .head(manifest_url)
            .header(reqwest::header::ACCEPT, MANIFEST_ACCEPT)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|error| {
                AppError::Internal(format!("Registry cleanup request failed: {error}"))
            })?;
        if response.status() == StatusCode::NOT_FOUND {
            continue;
        }
        if !response.status().is_success() {
            return Err(AppError::Conflict(format!(
                "Registry cleanup failed while resolving tag '{tag}': {}",
                response.status()
            )));
        }
        let digest = response
            .headers()
            .get("Docker-Content-Digest")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                AppError::Conflict(format!(
                    "Registry cleanup failed while resolving tag '{tag}': missing digest"
                ))
            })?;
        digests.insert(digest.to_owned());
    }
    for digest in digests {
        let manifest_url =
            registry_url(&base_url, repository_name, &format!("manifests/{digest}"))?;
        let response = client
            .delete(manifest_url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|error| {
                AppError::Internal(format!("Registry cleanup request failed: {error}"))
            })?;
        if !response.status().is_success() && response.status() != StatusCode::NOT_FOUND {
            return Err(AppError::Conflict(format!(
                "Registry cleanup failed while deleting image: {}",
                response.status()
            )));
        }
    }
    Ok(())
}

fn registry_url(base_url: &str, repository_name: &str, suffix: &str) -> Result<Url, AppError> {
    let mut url = Url::parse(base_url)
        .map_err(|error| AppError::Internal(format!("Invalid registry URL: {error}")))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| AppError::Internal("Registry URL cannot be a base URL".into()))?;
        segments.push("v2");
        segments.extend(repository_name.split('/'));
        segments.extend(suffix.split('/'));
    }
    Ok(url)
}

fn response(repository: &registry_repository::Model) -> RegistryRepositoryResponse {
    RegistryRepositoryResponse {
        id: repository.id,
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
