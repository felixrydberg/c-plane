use axum::{Json, extract::Path, http::StatusCode};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError,
    handlers::registry::{require_managed_registry, sign_repository_access},
    middleware::auth::AuthContext,
    models::entities::registry_repository,
    state::get_app_state,
};

use super::databases::{verify_org_access, verify_project_in_org};

// ponytail: single tags/list page; Link-follow up to MAX_TAGS instead of a cursor protocol.
const TAGS_PAGE_SIZE: u32 = 100;
const MAX_TAGS: usize = 1000;

#[derive(Serialize, ToSchema)]
pub struct RepositoryTagsResponse {
    pub tags: Vec<String>,
}

#[derive(Deserialize)]
struct DistributionTagsResponse {
    // Distribution encodes an empty tag list as explicit null, which
    // #[serde(default)] alone does not accept.
    #[serde(default)]
    tags: Option<Vec<String>>,
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
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, project_id, organization_id).await?;
    let repository = find_repository(tx, organization_id, project_id, repository_id).await?;
    scoped.commit().await?;

    let access =
        sign_repository_access(organization_id, project_id, repository.id, &repository.name, &[
            "pull",
        ])
        .await?;
    let base = registry_base_url()?;
    let client = &get_app_state().storage_client;
    let mut names = Vec::new();
    let mut url = format!(
        "{base}/v2/{}/tags/list?n={TAGS_PAGE_SIZE}",
        access.repository_name
    );
    loop {
        let response = client
            .get(&url)
            .bearer_auth(&access.token)
            .send()
            .await
            .map_err(|error| {
                AppError::ServiceUnavailable(format!("Container registry request failed: {error}"))
            })?;
        if response.status() == StatusCode::NOT_FOUND {
            break;
        }
        if !response.status().is_success() {
            tracing::warn!(
                "registry tags/list returned {} for repository {repository_id}",
                response.status()
            );
            return Err(AppError::ServiceUnavailable(format!(
                "Container registry returned {}",
                response.status()
            )));
        }
        let next = link_next(response.headers(), &base);
        let listed = response
            .json::<DistributionTagsResponse>()
            .await
            .map_err(|error| {
                AppError::ServiceUnavailable(format!("Invalid registry response: {error}"))
            })?;
        names.extend(listed.tags.unwrap_or_default());
        match next {
            Some(next_url) if names.len() < MAX_TAGS => url = next_url,
            _ => break,
        }
    }
    names.truncate(MAX_TAGS);
    Ok(Json(RepositoryTagsResponse { tags: names }))
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
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, project_id, organization_id).await?;
    let repository = find_repository(tx, organization_id, project_id, repository_id).await?;
    scoped.commit().await?;

    require_managed_registry(organization_id).await?;
    let access = sign_repository_access(
        organization_id,
        project_id,
        repository.id,
        &repository.name,
        &["delete"],
    )
    .await?;
    let base = registry_base_url()?;
    let client = &get_app_state().storage_client;
    let response = client
        .delete(format!(
            "{base}/v2/{}/manifests/{tag}",
            access.repository_name
        ))
        .bearer_auth(access.token)
        .send()
        .await
        .map_err(|error| {
            AppError::ServiceUnavailable(format!("Container registry request failed: {error}"))
        })?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(StatusCode::NO_CONTENT);
    }
    if !response.status().is_success() {
        tracing::warn!(
            "registry tag delete returned {} for repository {repository_id}",
            response.status()
        );
        return Err(AppError::ServiceUnavailable(format!(
            "Container registry returned {}",
            response.status()
        )));
    }
    let scoped = tenant_db.begin_scoped_transaction().await?;
    crate::services::events::record(
        scoped.connection(),
        organization_id,
        project_id,
        "registry-tag:deleted",
        json!({ "summary": format!("Deleted tag '{tag}' from registry repository '{}'", repository.name), "target_id": repository.id }),
        auth.actor_id,
    )
    .await?;
    scoped.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn find_repository(
    tx: &sea_orm::DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
    repository_id: Uuid,
) -> Result<registry_repository::Model, AppError> {
    registry_repository::Entity::find_by_id(repository_id)
        .filter(registry_repository::Column::OrganizationId.eq(organization_id))
        .filter(registry_repository::Column::ProjectId.eq(project_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Registry repository not found".into()))
}

fn link_next(headers: &reqwest::header::HeaderMap, base: &str) -> Option<String> {
    let link = headers
        .get(reqwest::header::LINK)?
        .to_str()
        .ok()?;
    for part in link.split(',') {
        let mut segments = part.trim().split(';');
        let target = segments.next()?.trim();
        if segments.any(|segment| segment.trim() == "rel=\"next\"") {
            let path = target.strip_prefix('<')?.strip_suffix('>')?;
            if path.starts_with("http://") || path.starts_with("https://") {
                return Some(path.to_string());
            }
            return Some(format!("{base}{path}"));
        }
    }
    None
}

fn registry_base_url() -> Result<String, AppError> {
    let value =
        env::var("REGISTRY_INTERNAL_URL").unwrap_or_else(|_| "http://registry:5000".into());
    reqwest::Url::parse(&value)
        .map_err(|_| AppError::Internal("REGISTRY_INTERNAL_URL is invalid".into()))?;
    Ok(value.trim_end_matches('/').to_string())
}

#[cfg(test)]
mod tests {
    use super::{DistributionTagsResponse, link_next, registry_base_url};

    #[test]
    fn trims_the_registry_base_url() {
        assert!(registry_base_url().is_ok());
    }

    #[test]
    fn accepts_an_explicit_null_tag_list() {
        // Distribution encodes an empty repository as {"name": "...", "tags": null}.
        let response: DistributionTagsResponse =
            serde_json::from_str(r#"{"name":"acme/test/acme","tags":null}"#).unwrap();
        assert!(response.tags.unwrap_or_default().is_empty());
    }

    #[test]
    fn follows_the_next_tags_page() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::LINK,
            r#"</v2/acme/api/tags/list?last=beta&n=100>; rel="next""#
                .parse()
                .unwrap(),
        );
        assert_eq!(
            link_next(&headers, "http://registry:5000"),
            Some("http://registry:5000/v2/acme/api/tags/list?last=beta&n=100".into())
        );
    }
}
