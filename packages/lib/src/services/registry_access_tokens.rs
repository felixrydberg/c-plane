use base64::Engine as _;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    entities::{
        managed_registry::{self, ManagedRegistryStatus},
        registry_access_token, registry_repository, registry_repository_grant,
    },
    error::AppError,
    services::events,
    tenant::TenantDatabase,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Permission {
    pub repository_id: Uuid,
    pub can_pull: bool,
    pub can_push: bool,
}
pub struct Created {
    pub token: registry_access_token::Model,
    pub plaintext: String,
}
pub struct Details {
    pub token: registry_access_token::Model,
    pub permissions: Vec<Permission>,
}

pub async fn delete_project_in_transaction(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
) -> Result<(), AppError> {
    registry_repository_grant::Entity::delete_many()
        .filter(registry_repository_grant::Column::OrganizationId.eq(organization_id))
        .filter(registry_repository_grant::Column::ProjectId.eq(project_id))
        .exec(tx)
        .await?;
    registry_access_token::Entity::delete_many()
        .filter(registry_access_token::Column::OrganizationId.eq(organization_id))
        .filter(registry_access_token::Column::ProjectId.eq(project_id))
        .exec(tx)
        .await?;
    Ok(())
}

pub fn valid_permissions(permissions: &[Permission]) -> bool {
    !permissions.is_empty()
        && permissions.iter().all(|p| p.can_pull || p.can_push)
        && permissions
            .iter()
            .map(|p| p.repository_id)
            .collect::<std::collections::HashSet<_>>()
            .len()
            == permissions.len()
}

pub async fn create(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
    actor_id: Uuid,
    name: String,
    permissions: &[Permission],
) -> Result<Created, AppError> {
    verify_owner(tenant_db, organization_id)?;
    let name = name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::Conflict(
            "Token name must be 1-100 characters".into(),
        ));
    }
    if !valid_permissions(permissions) {
        return Err(AppError::Conflict(
            "Select at least one valid repository permission".into(),
        ));
    }
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project(tx, organization_id, project_id).await?;
    require_active(tx, organization_id).await?;
    verify_repositories(tx, organization_id, project_id, permissions).await?;
    if registry_access_token::Entity::find()
        .filter(registry_access_token::Column::OrganizationId.eq(organization_id))
        .filter(registry_access_token::Column::ProjectId.eq(project_id))
        .filter(registry_access_token::Column::Name.eq(name))
        .filter(registry_access_token::Column::RevokedAt.is_null())
        .one(tx)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(
            "An active token with this name already exists".into(),
        ));
    }
    let plaintext = generate_token();
    let id = Uuid::new_v4();
    let token = registry_access_token::ActiveModel {
        id: Set(id),
        organization_id: Set(organization_id),
        project_id: Set(project_id),
        name: Set(name.into()),
        token_hash: Set(hex::encode(Sha256::digest(plaintext.as_bytes()))),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    replace_permissions(tx, organization_id, project_id, id, permissions).await?;
    events::record(tx, organization_id, project_id, "registry-access-token:created", serde_json::json!({"summary": format!("Created registry access token '{name}'"), "target_id": id, "repository_ids": permissions.iter().map(|permission| permission.repository_id).collect::<Vec<_>>() }), actor_id).await?;
    scoped.commit().await?;
    Ok(Created { token, plaintext })
}
pub async fn list(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
) -> Result<Vec<registry_access_token::Model>, AppError> {
    verify_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    verify_project(scoped.connection(), organization_id, project_id).await?;
    let rows = registry_access_token::Entity::find()
        .filter(registry_access_token::Column::OrganizationId.eq(organization_id))
        .filter(registry_access_token::Column::ProjectId.eq(project_id))
        .filter(registry_access_token::Column::RevokedAt.is_null())
        .order_by_desc(registry_access_token::Column::CreatedAt)
        .all(scoped.connection())
        .await?;
    scoped.commit().await?;
    Ok(rows)
}
pub async fn get(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
    token_id: Uuid,
) -> Result<Details, AppError> {
    verify_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project(tx, organization_id, project_id).await?;
    let token = active(tx, organization_id, project_id, token_id).await?;
    let permissions = registry_repository_grant::Entity::find()
        .filter(registry_repository_grant::Column::AccessTokenId.eq(token_id))
        .filter(registry_repository_grant::Column::ProjectId.eq(project_id))
        .all(tx)
        .await?
        .into_iter()
        .map(|p| Permission {
            repository_id: p.repository_id,
            can_pull: p.can_pull,
            can_push: p.can_push,
        })
        .collect();
    scoped.commit().await?;
    Ok(Details { token, permissions })
}
pub async fn update(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
    actor_id: Uuid,
    token_id: Uuid,
    permissions: &[Permission],
) -> Result<(), AppError> {
    verify_owner(tenant_db, organization_id)?;
    if !valid_permissions(permissions) {
        return Err(AppError::Conflict(
            "Select at least one valid repository permission".into(),
        ));
    }
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project(tx, organization_id, project_id).await?;
    require_active(tx, organization_id).await?;
    active(tx, organization_id, project_id, token_id).await?;
    verify_repositories(tx, organization_id, project_id, permissions).await?;
    replace_permissions(tx, organization_id, project_id, token_id, permissions).await?;
    events::record(tx, organization_id, project_id, "registry-access-token:updated", serde_json::json!({"summary":"Updated registry access token permissions","target_id":token_id, "repository_ids": permissions.iter().map(|permission| permission.repository_id).collect::<Vec<_>>() }), actor_id).await?;
    scoped.commit().await?;
    Ok(())
}
pub async fn revoke(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
    actor_id: Uuid,
    token_id: Uuid,
) -> Result<(), AppError> {
    verify_owner(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project(tx, organization_id, project_id).await?;
    let token = active(tx, organization_id, project_id, token_id).await?;
    let name = token.name.clone();
    let mut active = token.into_active_model();
    active.revoked_at = Set(Some(Utc::now().fixed_offset()));
    active.update(tx).await?;
    events::record(tx, organization_id, project_id, "registry-access-token:revoked", serde_json::json!({"summary": format!("Revoked registry access token '{name}'"), "target_id":token_id}), actor_id).await?;
    scoped.commit().await?;
    Ok(())
}
async fn active(
    tx: &DatabaseTransaction,
    org: Uuid,
    project: Uuid,
    id: Uuid,
) -> Result<registry_access_token::Model, AppError> {
    registry_access_token::Entity::find_by_id(id)
        .filter(registry_access_token::Column::OrganizationId.eq(org))
        .filter(registry_access_token::Column::ProjectId.eq(project))
        .filter(registry_access_token::Column::RevokedAt.is_null())
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Registry access token not found".into()))
}
async fn verify_repositories(
    tx: &DatabaseTransaction,
    org: Uuid,
    project: Uuid,
    permissions: &[Permission],
) -> Result<(), AppError> {
    let ids = permissions
        .iter()
        .map(|p| p.repository_id)
        .collect::<Vec<_>>();
    let rows = registry_repository::Entity::find()
        .filter(registry_repository::Column::Id.is_in(ids))
        .filter(registry_repository::Column::OrganizationId.eq(org))
        .filter(registry_repository::Column::ProjectId.eq(project))
        .all(tx)
        .await?;
    if rows.len() != permissions.len() {
        return Err(AppError::NotFound(
            "Repository not found in this project".into(),
        ));
    }
    Ok(())
}
async fn replace_permissions(
    tx: &DatabaseTransaction,
    org: Uuid,
    project: Uuid,
    token: Uuid,
    permissions: &[Permission],
) -> Result<(), AppError> {
    registry_repository_grant::Entity::delete_many()
        .filter(registry_repository_grant::Column::AccessTokenId.eq(token))
        .exec(tx)
        .await?;
    let rows = permissions
        .iter()
        .map(|p| registry_repository_grant::ActiveModel {
            id: Set(Uuid::new_v4()),
            organization_id: Set(org),
            project_id: Set(project),
            repository_id: Set(p.repository_id),
            access_token_id: Set(token),
            can_pull: Set(p.can_pull || p.can_push),
            can_push: Set(p.can_push),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    if !rows.is_empty() {
        registry_repository_grant::Entity::insert_many(rows)
            .exec(tx)
            .await?;
    }
    Ok(())
}
async fn verify_project(
    tx: &DatabaseTransaction,
    org: Uuid,
    project: Uuid,
) -> Result<(), AppError> {
    if crate::entities::project::Entity::find_by_id(project)
        .filter(crate::entities::project::Column::OrganizationId.eq(org))
        .one(tx)
        .await?
        .is_none()
    {
        return Err(AppError::NotFound(
            "Project not found in this organization".into(),
        ));
    }
    Ok(())
}
fn verify_access(db: &TenantDatabase, org: Uuid) -> Result<(), AppError> {
    if db.context.allowed_organizations.contains(&org) {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "You do not have access to this organization".into(),
        ))
    }
}
fn verify_owner(db: &TenantDatabase, org: Uuid) -> Result<(), AppError> {
    verify_access(db, org)?;
    if let Some(api_key_org) = db.context.api_key_organization_id {
        return if api_key_org == org {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "API key is not owned by this organization".into(),
            ))
        };
    }
    if db
        .context
        .organization_roles
        .get(&org)
        .is_some_and(|role| role == "owner")
    {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "Organization owner role required".into(),
        ))
    }
}
async fn require_active(tx: &DatabaseTransaction, org: Uuid) -> Result<(), AppError> {
    let registry = managed_registry::Entity::find_by_id(org)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::Conflict("Activate Managed Registry first".into()))?;
    if registry.status != ManagedRegistryStatus::Active {
        return Err(AppError::ServiceUnavailable(
            "Managed Registry is unavailable during maintenance".into(),
        ));
    }
    Ok(())
}
fn generate_token() -> String {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(Uuid::new_v4().as_bytes());
    bytes.extend_from_slice(Uuid::new_v4().as_bytes());
    format!(
        "cr_{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}
