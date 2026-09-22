use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
    sea_query::Expr,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{event, external_registry, project_revision_manifest, secret, secret::SecretScope},
    error::AppError,
    secrets::{self, Client},
    services::buckets::tenant_key,
    tenant::TenantDatabase,
};

const NAME_CONSTRAINT: &str = "external_registry_organization_name_uidx";
const HOST_USERNAME_CONSTRAINT: &str = "external_registry_organization_host_username_uidx";

#[derive(Clone, Copy)]
pub enum Provider {
    DockerHub,
    Github,
    Gitlab,
    GoogleArtifactRegistry,
    AwsEcr,
}

#[derive(Serialize, Deserialize)]
struct RegistrySecret {
    token: String,
}

pub fn required(value: String, name: &str) -> Result<String, AppError> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        Err(AppError::BadRequest(format!("{name} is required")))
    } else {
        Ok(value)
    }
}
pub fn required_secret(value: String) -> Result<String, AppError> {
    if value.trim().is_empty() {
        Err(AppError::BadRequest("Token is required".into()))
    } else {
        Ok(value)
    }
}
pub fn trusted_host(provider: Provider, host: Option<&str>) -> Result<String, AppError> {
    let (validator, expected): (fn(&str) -> bool, &str) = match provider {
        Provider::DockerHub => return Ok("docker.io".into()),
        Provider::Github => return Ok("ghcr.io".into()),
        Provider::Gitlab => return Ok("registry.gitlab.com".into()),
        Provider::GoogleArtifactRegistry => (
            |v| {
                v.strip_suffix("-docker.pkg.dev")
                    .is_some_and(valid_dns_label)
            },
            "Google Artifact Registry",
        ),
        Provider::AwsEcr => (valid_ecr_host, "AWS ECR"),
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
    account.len() == 12 && account.bytes().all(|b| b.is_ascii_digit()) && valid_dns_label(region)
}
fn valid_dns_label(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub async fn list(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
) -> Result<Vec<external_registry::Model>, AppError> {
    verify_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let rows = external_registry::Entity::find()
        .filter(external_registry::Column::OrganizationId.eq(organization_id))
        .order_by_asc(external_registry::Column::Name)
        .all(scoped.connection())
        .await?;
    scoped.commit().await?;
    Ok(rows)
}

pub async fn create(
    tenant_db: &TenantDatabase,
    secrets_client: &Client,
    organization_id: Uuid,
    actor_id: Uuid,
    name: String,
    provider: Provider,
    host: Option<&str>,
    username: String,
    token: String,
) -> Result<external_registry::Model, AppError> {
    verify_owner(tenant_db, organization_id)?;
    let name = required(name, "Name")?;
    let host = trusted_host(provider, host)?;
    let username = required(username, "Username")?;
    let token = required_secret(token)?;
    let id = Uuid::new_v4();
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    store_secret(tx, secrets_client, organization_id, id, &token).await?;
    let created = external_registry::ActiveModel {
        id: Set(id),
        organization_id: Set(organization_id),
        name: Set(name.clone()),
        host: Set(host),
        username: Set(username),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await
    .map_err(map_write_error)?;
    record_event(
        tx,
        organization_id,
        actor_id,
        "external-registry:created",
        json!({"summary": format!("Created external registry '{name}'"), "target_id": id}),
    )
    .await?;
    scoped.commit().await?;
    Ok(created)
}

pub async fn rename(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    actor_id: Uuid,
    registry_id: Uuid,
    name: String,
) -> Result<external_registry::Model, AppError> {
    verify_owner(tenant_db, organization_id)?;
    let name = required(name, "Name")?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let registry = find(tx, organization_id, registry_id).await?;
    let mut active: external_registry::ActiveModel = registry.into();
    active.name = Set(name.clone());
    active.updated_at = Set(Utc::now().fixed_offset());
    let updated = active.update(tx).await.map_err(map_write_error)?;
    record_event(tx, organization_id, actor_id, "external-registry:renamed", json!({"summary": format!("Renamed external registry to '{name}'"), "target_id": registry_id})).await?;
    scoped.commit().await?;
    Ok(updated)
}

pub async fn rotate(
    tenant_db: &TenantDatabase,
    secrets_client: &Client,
    organization_id: Uuid,
    actor_id: Uuid,
    registry_id: Uuid,
    token: String,
) -> Result<(), AppError> {
    verify_owner(tenant_db, organization_id)?;
    let token = required_secret(token)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let registry = find(tx, organization_id, registry_id).await?;
    let name = registry.name.clone();
    let mut active: external_registry::ActiveModel = registry.into();
    active.updated_at = Set(Utc::now().fixed_offset());
    active.update(tx).await.map_err(map_write_error)?;
    store_secret(tx, secrets_client, organization_id, registry_id, &token).await?;
    record_event(tx, organization_id, actor_id, "external-registry:token-rotated", json!({"summary": format!("Rotated token for external registry '{name}'"), "target_id": registry_id})).await?;
    scoped.commit().await?;
    Ok(())
}

pub async fn delete(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    actor_id: Uuid,
    registry_id: Uuid,
) -> Result<(), AppError> {
    verify_owner(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let registry = find(tx, organization_id, registry_id).await?;
    external_registry::Entity::find_by_id(registry_id)
        .filter(external_registry::Column::OrganizationId.eq(organization_id))
        .lock_exclusive()
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("External registry not found".into()))?;
    if manifest_references(tx, organization_id, registry_id).await? {
        return Err(AppError::Conflict(
            "Registry is used by one or more revisions".into(),
        ));
    }
    external_registry::Entity::delete_by_id(registry_id)
        .exec(tx)
        .await?;
    secret::Entity::delete_by_id(registry_id).exec(tx).await?;
    record_event(tx, organization_id, actor_id, "external-registry:deleted", json!({"summary": format!("Deleted external registry '{}'", registry.name), "target_id": registry_id})).await?;
    scoped.commit().await?;
    Ok(())
}

pub async fn find(
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

async fn store_secret(
    tx: &impl ConnectionTrait,
    client: &Client,
    organization_id: Uuid,
    id: Uuid,
    token: &str,
) -> Result<(), AppError> {
    let plaintext = serde_json::to_vec(&RegistrySecret {
        token: token.to_owned(),
    })
    .map_err(|e| AppError::Internal(e.to_string()))?;
    let ciphertext = secrets::encrypt(client, &tenant_key(organization_id), &plaintext).await?;
    if let Some(existing) = secret::Entity::find_by_id(id)
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
            id: Set(id),
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
async fn manifest_references(
    tx: &impl ConnectionTrait,
    organization_id: Uuid,
    id: Uuid,
) -> Result<bool, AppError> {
    Ok(project_revision_manifest::Entity::find()
        .filter(project_revision_manifest::Column::OrganizationId.eq(organization_id))
        .filter(Expr::cust_with_values(
            "external_registry_ids @> ARRAY[?]::uuid[]",
            [id],
        ))
        .one(tx)
        .await?
        .is_some())
}
async fn record_event(
    tx: &sea_orm::DatabaseTransaction,
    organization_id: Uuid,
    actor_id: Uuid,
    event_type: &str,
    payload: serde_json::Value,
) -> Result<(), AppError> {
    event::ActiveModel {
        id: Set(Uuid::new_v4()),
        organization_id: Set(organization_id),
        event_type: Set(event_type.into()),
        payload: Set(payload),
        system: Set(false),
        project_id: Set(None),
        actor_id: Set(Some(actor_id)),
        created_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;
    Ok(())
}
fn verify_access(tenant_db: &TenantDatabase, organization_id: Uuid) -> Result<(), AppError> {
    if tenant_db
        .context
        .allowed_organizations
        .contains(&organization_id)
    {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "You do not have access to this organization".into(),
        ))
    }
}

fn verify_owner(tenant_db: &TenantDatabase, organization_id: Uuid) -> Result<(), AppError> {
    verify_access(tenant_db, organization_id)?;
    if let Some(api_key_organization_id) = tenant_db.context.api_key_organization_id {
        return if api_key_organization_id == organization_id {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "API key is not owned by this organization".into(),
            ))
        };
    }
    if tenant_db
        .context
        .organization_roles
        .get(&organization_id)
        .is_some_and(|role| role == "owner")
    {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "Organization owner role required".into(),
        ))
    }
}

fn map_write_error(error: sea_orm::DbErr) -> AppError {
    let message = error.to_string();
    if message.contains(NAME_CONSTRAINT) {
        AppError::Conflict("An external registry with this name already exists".into())
    } else if message.contains(HOST_USERNAME_CONSTRAINT) {
        AppError::Conflict("This registry host and username already exist".into())
    } else {
        error.into()
    }
}
