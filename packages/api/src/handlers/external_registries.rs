use axum::{Json, extract::Path, http::StatusCode};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait, QueryFilter,
    QueryOrder, Set, Statement,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError, middleware::auth::AuthContext, models::entities::external_registry,
    state::get_app_state,
};

use super::{databases::verify_org_access, registry_access_tokens::record_event};

const DEPENDENCY_CONSTRAINT: &str = "project_container_version_external_registry_fk";
const NAME_CONSTRAINT: &str = "external_registry_organization_name_uidx";
const HOST_USERNAME_CONSTRAINT: &str = "external_registry_organization_host_username_uidx";

#[derive(Deserialize, ToSchema)]
pub struct CreateExternalRegistryRequest {
    pub name: String,
    pub host: String,
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

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/registry/external-registries",
    params(("organization_id" = Uuid, Path, description = "Organization ID")),
    responses((status = 200, body = Vec<ExternalRegistryResponse>)),
    tag = "registry",
)]
pub async fn list_external_registries(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
) -> Result<Json<Vec<ExternalRegistryResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let registries = external_registry::Entity::find()
        .filter(external_registry::Column::OrganizationId.eq(organization_id))
        .order_by_asc(external_registry::Column::Name)
        .all(scoped.connection())
        .await?;
    scoped.commit().await?;
    Ok(Json(registries.iter().map(response).collect()))
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/registry/external-registries",
    request_body = CreateExternalRegistryRequest,
    params(("organization_id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 201, body = ExternalRegistryResponse),
        (status = 400, body = crate::errors::ErrorResponse),
        (status = 409, body = crate::errors::ErrorResponse),
    ),
    tag = "registry",
)]
pub async fn create_external_registry(
    AuthContext { tenant_db, auth }: AuthContext,
    Path(organization_id): Path<Uuid>,
    Json(body): Json<CreateExternalRegistryRequest>,
) -> Result<(StatusCode, Json<ExternalRegistryResponse>), AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let name = required(body.name, "Name")?;
    let host = normalize_registry_host(&body.host)?;
    let username = required(body.username, "Username")?;
    let token = required_secret(body.token)?;
    let registry_id = Uuid::new_v4();
    let secrets = secret_service()?;
    secrets.store(organization_id, registry_id, &token).await?;

    let result = async {
        let scoped = tenant_db.begin_scoped_transaction().await?;
        let tx = scoped.connection();
        let created = external_registry::ActiveModel {
            id: Set(registry_id),
            organization_id: Set(organization_id),
            name: Set(name.clone()),
            host: Set(host),
            username: Set(username),
            created_at: Set(Utc::now().fixed_offset()),
            updated_at: Set(Utc::now().fixed_offset()),
        }
        .insert(tx)
        .await
        .map_err(map_registry_write_error)?;
        record_event(
            tx,
            organization_id,
            auth.actor_id,
            "external-registry:created",
            json!({"summary": format!("Created external registry '{}'", created.name), "target_id": created.id}),
        )
        .await?;
        scoped.commit().await?;
        Ok::<_, AppError>(created)
    }
    .await;

    match result {
        Ok(created) => Ok((StatusCode::CREATED, Json(response(&created)))),
        Err(error) => {
            if let Err(cleanup_error) = secrets.delete(organization_id, registry_id).await {
                tracing::warn!(%cleanup_error, %organization_id, %registry_id, "failed to clean up external registry secret after create failure");
                if let Err(job_error) = async {
                    let scoped = tenant_db.begin_scoped_transaction().await?;
                    enqueue_external_registry_secret_cleanup(
                        scoped.connection(),
                        organization_id,
                        registry_id,
                    )
                    .await?;
                    scoped.commit().await?;
                    Ok::<_, AppError>(())
                }
                .await
                {
                    tracing::warn!(%job_error, %organization_id, %registry_id, "failed to queue external registry secret cleanup after create failure");
                }
            }
            Err(error)
        }
    }
}

#[utoipa::path(
    patch,
    path = "/api/organization/{organization_id}/registry/external-registries/{registry_id}",
    request_body = RenameExternalRegistryRequest,
    params(("organization_id" = Uuid, Path), ("registry_id" = Uuid, Path)),
    responses(
        (status = 200, body = ExternalRegistryResponse),
        (status = 400, body = crate::errors::ErrorResponse),
        (status = 404),
        (status = 409, body = crate::errors::ErrorResponse),
    ),
    tag = "registry",
)]
pub async fn rename_external_registry(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, registry_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<RenameExternalRegistryRequest>,
) -> Result<Json<ExternalRegistryResponse>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let name = required(body.name, "Name")?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let registry = find_registry(tx, organization_id, registry_id).await?;
    let mut active: external_registry::ActiveModel = registry.into();
    active.name = Set(name.clone());
    active.updated_at = Set(Utc::now().fixed_offset());
    let updated = active.update(tx).await.map_err(map_registry_write_error)?;
    record_event(
        tx,
        organization_id,
        auth.actor_id,
        "external-registry:renamed",
        json!({"summary": format!("Renamed external registry to '{name}'"), "target_id": registry_id}),
    )
    .await?;
    scoped.commit().await?;
    Ok(Json(response(&updated)))
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/registry/external-registries/{registry_id}/rotate-token",
    request_body = RotateExternalRegistryTokenRequest,
    params(("organization_id" = Uuid, Path), ("registry_id" = Uuid, Path)),
    responses(
        (status = 204),
        (status = 400, body = crate::errors::ErrorResponse),
        (status = 404),
        (status = 409, body = crate::errors::ErrorResponse),
    ),
    tag = "registry",
)]
pub async fn rotate_external_registry_token(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, registry_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<RotateExternalRegistryTokenRequest>,
) -> Result<StatusCode, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let token = required_secret(body.token)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let registry = find_registry(tx, organization_id, registry_id).await?;
    let registry_name = registry.name.clone();
    let mut active: external_registry::ActiveModel = registry.into();
    active.updated_at = Set(Utc::now().fixed_offset());
    active.update(tx).await.map_err(map_registry_write_error)?;
    record_event(
        tx,
        organization_id,
        auth.actor_id,
        "external-registry:token-rotated",
        json!({"summary": format!("Rotated token for external registry '{registry_name}'"), "target_id": registry_id}),
    )
    .await?;
    scoped.commit().await?;
    secret_service()?
        .store(organization_id, registry_id, &token)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/api/organization/{organization_id}/registry/external-registries/{registry_id}",
    params(("organization_id" = Uuid, Path), ("registry_id" = Uuid, Path)),
    responses((status = 204), (status = 404), (status = 409, body = crate::errors::ErrorResponse)),
    tag = "registry",
)]
pub async fn delete_external_registry(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, registry_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let registry = find_registry(tx, organization_id, registry_id).await?;
    external_registry::Entity::delete_by_id(registry_id)
        .exec(tx)
        .await
        .map_err(map_registry_delete_error)?;
    enqueue_external_registry_secret_cleanup(tx, organization_id, registry_id).await?;
    record_event(
        tx,
        organization_id,
        auth.actor_id,
        "external-registry:deleted",
        json!({"summary": format!("Deleted external registry '{}'", registry.name), "target_id": registry_id}),
    )
    .await?;
    scoped.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn enqueue_external_registry_secret_cleanup(
    tx: &impl ConnectionTrait,
    organization_id: Uuid,
    registry_id: Uuid,
) -> Result<(), AppError> {
    tx.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO worker_job (id, organization_id, queue_name, job_type, dedupe_key, payload, max_attempts) VALUES ($1::uuid, $2::uuid, 'secrets', 'external_registry_secret_cleanup', $3, $4::jsonb, 8)",
        vec![
            Uuid::new_v4().into(),
            organization_id.into(),
            format!("external-registry-secret:{registry_id}").into(),
            json!({"organization_id": organization_id, "registry_id": registry_id}).to_string().into(),
        ],
    ))
    .await?;
    Ok(())
}

pub async fn find_registry(
    tx: &impl ConnectionTrait,
    organization_id: Uuid,
    registry_id: Uuid,
) -> Result<external_registry::Model, AppError> {
    external_registry::Entity::find_by_id(registry_id)
        .filter(external_registry::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("External registry not found".into()))
}

pub fn normalize_registry_host(value: &str) -> Result<String, AppError> {
    let value = value.trim().trim_end_matches('/');
    if value.to_ascii_lowercase().starts_with("http://") {
        return Err(AppError::BadRequest(
            "External registries must use HTTPS".into(),
        ));
    }
    let value = if value
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
    {
        &value[8..]
    } else {
        value
    };
    if value.is_empty()
        || value.contains("://")
        || value.contains('/')
        || value.contains('@')
        || value.contains('?')
        || value.contains('#')
    {
        return Err(AppError::BadRequest(
            "Registry host must be a hostname with an optional port".into(),
        ));
    }
    let url = reqwest::Url::parse(&format!("https://{value}"))
        .map_err(|_| AppError::BadRequest("Invalid registry host".into()))?;
    let host = url
        .host_str()
        .ok_or_else(|| AppError::BadRequest("Invalid registry host".into()))?
        .to_ascii_lowercase();
    let canonical = match host.as_str() {
        "index.docker.io" | "registry-1.docker.io" => "docker.io".to_string(),
        _ => host,
    };
    Ok(url
        .port()
        .map_or(canonical.clone(), |port| format!("{canonical}:{port}")))
}

pub fn image_registry_host(image: &str) -> Result<String, AppError> {
    let image = image.trim();
    if image.is_empty() || image.contains("://") || image.starts_with('/') {
        return Err(AppError::BadRequest("Invalid image reference".into()));
    }
    let name = image.split('@').next().unwrap_or_default();
    let first = name.split('/').next().unwrap_or_default();
    if first.is_empty() {
        return Err(AppError::BadRequest("Invalid image reference".into()));
    }
    if !name.contains('/') || !(first.contains('.') || first.contains(':') || first == "localhost")
    {
        return Ok("docker.io".into());
    }
    normalize_registry_host(first)
}

fn secret_service()
-> Result<crate::services::external_registry_tokens::ExternalRegistryTokenClient, AppError> {
    get_app_state()
        .external_registry_tokens
        .ok_or_else(|| AppError::Internal("Control-plane secret service is not configured".into()))
}

fn required(value: String, name: &str) -> Result<String, AppError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(AppError::BadRequest(format!("{name} is required")))
    } else {
        Ok(value)
    }
}

fn required_secret(value: String) -> Result<String, AppError> {
    if value.trim().is_empty() {
        Err(AppError::BadRequest("Token is required".into()))
    } else {
        Ok(value)
    }
}

fn map_registry_write_error(error: sea_orm::DbErr) -> AppError {
    let message = error.to_string();
    if message.contains(NAME_CONSTRAINT) {
        AppError::Conflict("An external registry with this name already exists".into())
    } else if message.contains(HOST_USERNAME_CONSTRAINT) {
        AppError::Conflict("This registry host and username already exist".into())
    } else {
        error.into()
    }
}

fn map_registry_delete_error(error: sea_orm::DbErr) -> AppError {
    if error.to_string().contains(DEPENDENCY_CONSTRAINT) {
        AppError::Conflict("Registry is used by one or more container versions".into())
    } else {
        error.into()
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

#[cfg(test)]
mod tests {
    use super::{
        DEPENDENCY_CONSTRAINT, image_registry_host, map_registry_delete_error,
        normalize_registry_host, required_secret,
    };
    use crate::errors::AppError;

    #[test]
    fn normalizes_registry_hosts() {
        assert_eq!(normalize_registry_host("GHCR.IO/").unwrap(), "ghcr.io");
        assert_eq!(
            normalize_registry_host("https://GHCR.IO/").unwrap(),
            "ghcr.io"
        );
        assert_eq!(
            normalize_registry_host("registry-1.docker.io").unwrap(),
            "docker.io"
        );
        assert_eq!(
            normalize_registry_host("example.com:5443").unwrap(),
            "example.com:5443"
        );
        assert!(normalize_registry_host("http://example.com").is_err());
        assert_eq!(
            normalize_registry_host("127.0.0.1:5000").unwrap(),
            "127.0.0.1:5000"
        );
    }

    #[test]
    fn extracts_canonical_image_registry() {
        assert_eq!(image_registry_host("nginx:latest").unwrap(), "docker.io");
        assert_eq!(
            image_registry_host("library/nginx:latest").unwrap(),
            "docker.io"
        );
        assert_eq!(
            image_registry_host("ghcr.io/acme/api:v1").unwrap(),
            "ghcr.io"
        );
        assert_eq!(
            image_registry_host("registry.example.com:5443/acme/api:v1").unwrap(),
            "registry.example.com:5443"
        );
    }

    #[test]
    fn validates_without_normalizing_tokens() {
        assert_eq!(required_secret(" token ".into()).unwrap(), " token ");
        assert!(required_secret("  ".into()).is_err());
    }

    #[test]
    fn maps_only_the_registry_dependency_constraint_to_conflict() {
        assert!(matches!(
            map_registry_delete_error(sea_orm::DbErr::Custom(DEPENDENCY_CONSTRAINT.into())),
            AppError::Conflict(_)
        ));
        assert!(matches!(
            map_registry_delete_error(sea_orm::DbErr::Custom("another constraint".into())),
            AppError::Internal(_)
        ));
    }
}
