use axum::{Json, extract::Path, http::StatusCode};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError,
    middleware::auth::AuthContext,
    models::entities::{external_registry, secret},
    services::buckets::tenant_key,
    state::get_app_state,
};
use lib::entities::secret::SecretScope;

use super::{
    databases::{verify_org_access, verify_org_owner},
    registry_access_tokens::record_event,
};

const DEPENDENCY_CONSTRAINT: &str = "container_version_external_registry_fk";
const NAME_CONSTRAINT: &str = "external_registry_organization_name_uidx";
const HOST_USERNAME_CONSTRAINT: &str = "external_registry_organization_host_username_uidx";

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
    verify_org_owner(&tenant_db, organization_id)?;
    let name = required(body.name, "Name")?;
    let host = trusted_registry_host(body.provider, body.host.as_deref())?;
    let username = required(body.username, "Username")?;
    let token = required_secret(body.token)?;
    let registry_id = Uuid::new_v4();

    let result = async {
        let scoped = tenant_db.begin_scoped_transaction().await?;
        let tx = scoped.connection();
        store_secret(tx, organization_id, registry_id, &token).await?;
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
        Err(error) => Err(error),
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
    verify_org_owner(&tenant_db, organization_id)?;
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
    verify_org_owner(&tenant_db, organization_id)?;
    let token = required_secret(body.token)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let registry = find_registry(tx, organization_id, registry_id).await?;
    let registry_name = registry.name.clone();
    let mut active: external_registry::ActiveModel = registry.into();
    active.updated_at = Set(Utc::now().fixed_offset());
    active.update(tx).await.map_err(map_registry_write_error)?;
    store_secret(tx, organization_id, registry_id, &token).await?;
    record_event(
        tx,
        organization_id,
        auth.actor_id,
        "external-registry:token-rotated",
        json!({"summary": format!("Rotated token for external registry '{registry_name}'"), "target_id": registry_id}),
    )
    .await?;
    scoped.commit().await?;
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
    verify_org_owner(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let registry = find_registry(tx, organization_id, registry_id).await?;
    external_registry::Entity::delete_by_id(registry_id)
        .exec(tx)
        .await
        .map_err(map_registry_delete_error)?;
    secret::Entity::delete_by_id(registry_id).exec(tx).await?;
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

fn trusted_registry_host(
    provider: ExternalRegistryProvider,
    host: Option<&str>,
) -> Result<String, AppError> {
    let (validator, expected): (fn(&str) -> bool, &str) = match provider {
        ExternalRegistryProvider::DockerHub => return Ok("docker.io".into()),
        ExternalRegistryProvider::Github => return Ok("ghcr.io".into()),
        ExternalRegistryProvider::Gitlab => return Ok("registry.gitlab.com".into()),
        ExternalRegistryProvider::GoogleArtifactRegistry => (
            valid_google_artifact_registry_host,
            "Google Artifact Registry",
        ),
        ExternalRegistryProvider::AwsEcr => (valid_ecr_host, "AWS ECR"),
    };
    let host = host.unwrap_or_default().trim().to_ascii_lowercase();
    if validator(&host) {
        Ok(host)
    } else {
        Err(AppError::BadRequest(format!(
            "A valid {expected} host is required"
        )))
    }
}

fn valid_google_artifact_registry_host(host: &str) -> bool {
    host.strip_suffix("-docker.pkg.dev")
        .is_some_and(valid_dns_label)
}

fn valid_ecr_host(host: &str) -> bool {
    let Some(host) = host
        .strip_suffix(".amazonaws.com")
        .or_else(|| host.strip_suffix(".amazonaws.com.cn"))
    else {
        return false;
    };
    let Some((account, region)) = host.split_once(".dkr.ecr.") else {
        return false;
    };
    account.len() == 12
        && account.bytes().all(|byte| byte.is_ascii_digit())
        && valid_dns_label(region)
}

fn valid_dns_label(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

#[derive(Serialize, Deserialize)]
struct ExternalRegistrySecret {
    token: String,
}

// ponytail: secret id == registry id, no extra column/migration for one token.
async fn store_secret(
    tx: &impl ConnectionTrait,
    organization_id: Uuid,
    registry_id: Uuid,
    token: &str,
) -> Result<(), AppError> {
    let plaintext = serde_json::to_vec(&ExternalRegistrySecret {
        token: token.to_owned(),
    })
    .map_err(|error| AppError::Internal(error.to_string()))?;
    let ciphertext =
        lib::secrets::encrypt(&get_app_state().secrets, &tenant_key(organization_id), &plaintext)
            .await?;
    if let Some(existing) = secret::Entity::find_by_id(registry_id)
        .filter(secret::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
    {
        let mut active: secret::ActiveModel = existing.into();
        active.ciphertext = Set(ciphertext);
        active.updated_at = Set(Utc::now().fixed_offset());
        active.update(tx).await?;
    } else {
        secret::ActiveModel {
            id: Set(registry_id),
            scope: Set(SecretScope::Tenant),
            organization_id: Set(Some(organization_id)),
            ciphertext: Set(ciphertext),
            ..Default::default()
        }
        .insert(tx)
        .await?;
    }
    Ok(())
}

pub async fn load_secret(organization_id: Uuid, registry_id: Uuid) -> Result<String, AppError> {
    let row = secret::Entity::find_by_id(registry_id)
        .filter(secret::Column::OrganizationId.eq(organization_id))
        .one(get_app_state().identity_db.connection())
        .await?
        .filter(|row| row.scope == SecretScope::Tenant)
        .filter(|row| row.organization_id == Some(organization_id))
        .ok_or_else(|| {
            AppError::Conflict("External registry credentials are unavailable".into())
        })?;
    let plaintext =
        lib::secrets::decrypt(&get_app_state().secrets, &tenant_key(organization_id), &row.ciphertext)
            .await?;
    let secret: ExternalRegistrySecret = serde_json::from_slice(&plaintext)
        .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(secret.token)
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
        DEPENDENCY_CONSTRAINT, ExternalRegistryProvider, map_registry_delete_error,
        required_secret, trusted_registry_host,
    };
    use crate::errors::AppError;

    #[test]
    fn allows_only_supported_private_registry_hosts() {
        assert_eq!(
            trusted_registry_host(ExternalRegistryProvider::DockerHub, None).unwrap(),
            "docker.io"
        );
        assert_eq!(
            trusted_registry_host(ExternalRegistryProvider::Github, None).unwrap(),
            "ghcr.io"
        );
        assert_eq!(
            trusted_registry_host(
                ExternalRegistryProvider::GoogleArtifactRegistry,
                Some("EUROPE-WEST1-DOCKER.PKG.DEV"),
            )
            .unwrap(),
            "europe-west1-docker.pkg.dev"
        );
        assert_eq!(
            trusted_registry_host(
                ExternalRegistryProvider::AwsEcr,
                Some("123456789012.dkr.ecr.eu-north-1.amazonaws.com"),
            )
            .unwrap(),
            "123456789012.dkr.ecr.eu-north-1.amazonaws.com"
        );
        assert!(
            trusted_registry_host(
                ExternalRegistryProvider::GoogleArtifactRegistry,
                Some("registry.example.com"),
            )
            .is_err()
        );
        assert!(
            trusted_registry_host(ExternalRegistryProvider::AwsEcr, Some("127.0.0.1:5000"),)
                .is_err()
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
