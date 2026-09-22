use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    entities::{
        managed_registry::{self, ManagedRegistryStatus},
        project, registry_repository,
    },
    error::AppError,
    operation::{Operation, registry_repository_delete::RegistryRepositoryDelete},
    services::events,
    tenant::TenantDatabase,
};

pub struct CreateInput {
    pub project_id: Uuid,
    pub name: String,
}

pub fn valid_name(name: &str) -> bool {
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

pub async fn list(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
) -> Result<Vec<registry_repository::Model>, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    verify_project(scoped.connection(), organization_id, project_id).await?;
    let rows = registry_repository::Entity::find()
        .filter(registry_repository::Column::ProjectId.eq(project_id))
        .order_by_asc(registry_repository::Column::Name)
        .all(scoped.connection())
        .await?;
    scoped.commit().await?;
    Ok(rows)
}

pub async fn create(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    actor_id: Uuid,
    input: CreateInput,
) -> Result<registry_repository::Model, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let name = input.name.trim().to_owned();
    if !valid_name(&name) {
        return Err(AppError::Conflict("Repository names must use lowercase letters, numbers, dots, underscores, dashes, and slashes".into()));
    }
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project(tx, organization_id, input.project_id).await?;
    let managed = managed_registry::Entity::find_by_id(organization_id)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::Conflict("Activate Managed Registry first".into()))?;
    if managed.status != ManagedRegistryStatus::Active {
        return Err(AppError::ServiceUnavailable(
            "Managed Registry is unavailable during maintenance".into(),
        ));
    }
    if registry_repository::Entity::find()
        .filter(registry_repository::Column::ProjectId.eq(input.project_id))
        .filter(registry_repository::Column::Name.eq(&name))
        .one(tx)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict("Repository already exists".into()));
    }
    let created = registry_repository::ActiveModel {
        id: Set(Uuid::new_v4()),
        organization_id: Set(organization_id),
        project_id: Set(input.project_id),
        name: Set(name.clone()),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    events::record(tx, organization_id, input.project_id, "registry-repository:created", json!({"summary": format!("Created registry repository '{name}'"), "target_id": created.id}), actor_id).await?;
    scoped.commit().await?;
    Ok(created)
}

pub async fn delete(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    actor_id: Uuid,
    project_id: Uuid,
    repository_id: Uuid,
) -> Result<(), AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    delete_in_transaction(
        scoped.connection(),
        organization_id,
        actor_id,
        project_id,
        repository_id,
    )
    .await?;
    scoped.commit().await?;
    Ok(())
}

pub async fn delete_in_transaction(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    actor_id: Uuid,
    project_id: Uuid,
    repository_id: Uuid,
) -> Result<(), AppError> {
    let repository = registry_repository::Entity::find_by_id(repository_id)
        .filter(registry_repository::Column::OrganizationId.eq(organization_id))
        .filter(registry_repository::Column::ProjectId.eq(project_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Registry repository not found".into()))?;
    Operation::<RegistryRepositoryDelete>::new(tx, organization_id, project_id, repository.id)
        .await?;
    events::record(tx, organization_id, project_id, "registry-repository:deleted", json!({"summary": format!("Deleted registry repository '{}'", repository.name), "target_id": repository.id}), actor_id).await?;
    registry_repository::Entity::delete_by_id(repository.id)
        .exec(tx)
        .await?;
    Ok(())
}

pub async fn delete_project_in_transaction(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
    actor_id: Uuid,
) -> Result<u64, AppError> {
    let rows = registry_repository::Entity::find()
        .filter(registry_repository::Column::OrganizationId.eq(organization_id))
        .filter(registry_repository::Column::ProjectId.eq(project_id))
        .all(tx)
        .await?;
    let repository_ids = rows.iter().map(|row| row.id).collect::<Vec<_>>();
    Operation::<RegistryRepositoryDelete>::new_many(
        tx,
        organization_id,
        project_id,
        repository_ids,
    )
    .await?;
    let count = rows.len() as u64;
    if !rows.is_empty() {
        let events = rows.into_iter().map(|row| crate::entities::event::ActiveModel { id: Set(Uuid::new_v4()), organization_id: Set(organization_id), project_id: Set(Some(project_id)), event_type: Set("registry-repository:deleted".into()), payload: Set(json!({"summary": format!("Deleted registry repository '{}'", row.name), "target_id": row.id})), system: Set(false), actor_id: Set(Some(actor_id)), created_at: Set(chrono::Utc::now().fixed_offset()) }).collect::<Vec<_>>();
        crate::entities::event::Entity::insert_many(events)
            .exec(tx)
            .await?;
        registry_repository::Entity::delete_many()
            .filter(registry_repository::Column::OrganizationId.eq(organization_id))
            .filter(registry_repository::Column::ProjectId.eq(project_id))
            .exec(tx)
            .await?;
    }
    Ok(count)
}

async fn verify_project(
    tx: &impl sea_orm::ConnectionTrait,
    organization_id: Uuid,
    project_id: Uuid,
) -> Result<(), AppError> {
    if project::Entity::find_by_id(project_id)
        .filter(project::Column::OrganizationId.eq(organization_id))
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

fn verify_org_access(tenant_db: &TenantDatabase, organization_id: Uuid) -> Result<(), AppError> {
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
