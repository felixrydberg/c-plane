use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::Deserialize;
use serde_json::json;
use std::env;
use uuid::Uuid;

use crate::{
    entities::{
        managed_registry::{self, ManagedRegistryStatus},
        registry_repository,
    },
    error::AppError,
    services::{events, registry},
    tenant::TenantDatabase,
};

const TAGS_PAGE_SIZE: u32 = 100;
const MAX_TAGS: usize = 1000;

pub struct Context<'a> {
    pub identity_db: &'a DatabaseConnection,
    pub client: &'a reqwest::Client,
    pub token_ttl_seconds: u64,
}

#[derive(Deserialize)]
struct DistributionTagsResponse {
    #[serde(default)]
    tags: Option<Vec<String>>,
}

pub async fn list(
    tenant_db: &TenantDatabase,
    context: &Context<'_>,
    organization_id: Uuid,
    project_id: Uuid,
    repository_id: Uuid,
) -> Result<Vec<String>, AppError> {
    let repository =
        find_scoped_repository(tenant_db, organization_id, project_id, repository_id).await?;
    let access = registry::sign_repository_access(
        context.identity_db,
        organization_id,
        project_id,
        repository.id,
        &repository.name,
        &["pull"],
        context.token_ttl_seconds,
    )
    .await?;
    let base = registry_base_url()?;
    let mut names = Vec::new();
    let mut url = format!(
        "{base}/v2/{}/tags/list?n={TAGS_PAGE_SIZE}",
        access.repository_name
    );
    loop {
        let response = context
            .client
            .get(&url)
            .bearer_auth(&access.token)
            .send()
            .await
            .map_err(|error| {
                AppError::ServiceUnavailable(format!("Container registry request failed: {error}"))
            })?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            break;
        }
        if !response.status().is_success() {
            tracing::warn!(%repository_id, status = %response.status(), "registry tags/list failed");
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
    Ok(names)
}

pub async fn delete(
    tenant_db: &TenantDatabase,
    context: &Context<'_>,
    organization_id: Uuid,
    project_id: Uuid,
    repository_id: Uuid,
    tag: &str,
    actor_id: Uuid,
) -> Result<(), AppError> {
    let repository =
        find_scoped_repository(tenant_db, organization_id, project_id, repository_id).await?;
    let registry = managed_registry::Entity::find_by_id(organization_id)
        .one(context.identity_db)
        .await?
        .ok_or_else(|| AppError::Conflict("Activate Managed Registry first".into()))?;
    if registry.status != ManagedRegistryStatus::Active {
        return Err(AppError::ServiceUnavailable(
            "Managed Registry is unavailable during maintenance".into(),
        ));
    }
    let access = crate::services::registry::sign_repository_access(
        context.identity_db,
        organization_id,
        project_id,
        repository.id,
        &repository.name,
        &["delete"],
        context.token_ttl_seconds,
    )
    .await?;
    let base = registry_base_url()?;
    let response = context
        .client
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
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(());
    }
    if !response.status().is_success() {
        tracing::warn!(%repository_id, status = %response.status(), "registry tag delete failed");
        return Err(AppError::ServiceUnavailable(format!(
            "Container registry returned {}",
            response.status()
        )));
    }
    let scoped = tenant_db.begin_scoped_transaction().await?;
    events::record(scoped.connection(), organization_id, project_id, "registry-tag:deleted",
        json!({ "summary": format!("Deleted tag '{tag}' from registry repository '{}'", repository.name), "target_id": repository.id }),
        actor_id).await?;
    scoped.commit().await?;
    Ok(())
}

async fn find_scoped_repository(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
    repository_id: Uuid,
) -> Result<registry_repository::Model, AppError> {
    if !tenant_db
        .context
        .allowed_organizations
        .contains(&organization_id)
    {
        return Err(AppError::Forbidden(
            "You do not have access to this organization".into(),
        ));
    }
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    if crate::entities::project::Entity::find_by_id(project_id)
        .filter(crate::entities::project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .is_none()
    {
        return Err(AppError::NotFound(
            "Project not found in this organization".into(),
        ));
    }
    let repository = registry_repository::Entity::find_by_id(repository_id)
        .filter(registry_repository::Column::OrganizationId.eq(organization_id))
        .filter(registry_repository::Column::ProjectId.eq(project_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Registry repository not found".into()))?;
    scoped.commit().await?;
    Ok(repository)
}

fn link_next(headers: &reqwest::header::HeaderMap, base: &str) -> Option<String> {
    let link = headers.get(reqwest::header::LINK)?.to_str().ok()?;
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
    let value = env::var("REGISTRY_INTERNAL_URL").unwrap_or_else(|_| "http://registry:5000".into());
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
