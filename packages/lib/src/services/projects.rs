use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::{
    entities::{project, project_environment},
    error::AppError,
    secrets::Client,
    services::s3_providers::S3ProviderClient,
    services::{
        containers, events,
        postgres_databases::{self, ServiceContext},
        project_environments, registry_access_tokens, registry_repositories, storage_access_tokens,
        storage_buckets,
    },
    tenant::TenantDatabase,
};

pub struct ProjectEnvironment {
    pub id: Uuid,
    pub name: String,
    pub is_preview: bool,
    pub draft_timeline: Uuid,
    pub deployed_timeline: Uuid,
    pub is_default: bool,
}
pub struct ProjectRecord {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub default_environment_id: Option<Uuid>,
    pub main_environment: Option<ProjectEnvironment>,
    pub created_at: String,
    pub updated_at: String,
}
pub struct ProjectPage {
    pub data: Vec<ProjectRecord>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
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

fn verify_org_owner(tenant_db: &TenantDatabase, organization_id: Uuid) -> Result<(), AppError> {
    verify_org_access(tenant_db, organization_id)?;
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

fn record(row: project::Model, environment: Option<project_environment::Model>) -> ProjectRecord {
    let default_id = row.default_environment_id;
    ProjectRecord {
        id: row.id,
        organization_id: row.organization_id,
        name: row.name,
        default_environment_id: default_id,
        main_environment: environment.map(|e| ProjectEnvironment {
            id: e.id,
            name: e.name,
            is_preview: e.is_preview,
            draft_timeline: e.draft_timeline,
            deployed_timeline: e.deployed_timeline,
            is_default: default_id == Some(e.id),
        }),
        created_at: row.created_at.to_string(),
        updated_at: row.updated_at.to_string(),
    }
}
async fn verify_project(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
) -> Result<project::Model, AppError> {
    project::Entity::find_by_id(project_id)
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".into()))
}

pub async fn create(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    actor_id: Uuid,
    name: &str,
) -> Result<ProjectRecord, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name is required".into()));
    }
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let project_id = Uuid::new_v4();
    let environment_id = Uuid::new_v4();
    let created = project::ActiveModel {
        id: Set(project_id),
        organization_id: Set(organization_id),
        name: Set(name.into()),
        default_environment_id: Set(None),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    let environment = project_environments::create_initial_in_transaction(
        tx,
        organization_id,
        project_id,
        environment_id,
    )
    .await?;
    let mut active: project::ActiveModel = created.into();
    active.default_environment_id = Set(Some(environment_id));
    active.updated_at = Set(Utc::now().fixed_offset());
    let updated = active.update(tx).await?;
    events::record(tx,organization_id,project_id,"project:created",serde_json::json!({"summary":format!("Created project '{}'",updated.name),"target_id":project_id.to_string()}),actor_id).await?;
    scoped.commit().await?;
    Ok(record(updated, Some(environment)))
}

pub async fn list(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    search: Option<&str>,
    page: u64,
    per_page: u64,
) -> Result<ProjectPage, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let mut select =
        project::Entity::find().filter(project::Column::OrganizationId.eq(organization_id));
    if let Some(search) = search.filter(|s| !s.trim().is_empty()) {
        select = select.filter(project::Column::Name.contains(search.trim()));
    }
    let total = select.clone().count(tx).await?;
    let rows = select
        .order_by_asc(project::Column::Name)
        .find_also_related(project_environment::Entity)
        .paginate(tx, per_page)
        .fetch_page(page.saturating_sub(1))
        .await?;
    scoped.commit().await?;
    Ok(ProjectPage {
        data: rows.into_iter().map(|(p, e)| record(p, e)).collect(),
        total,
        page,
        per_page,
    })
}

pub async fn get(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
) -> Result<ProjectRecord, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let p = verify_project(tx, organization_id, project_id).await?;
    let environment = match p.default_environment_id {
        Some(id) => project_environment::Entity::find_by_id(id).one(tx).await?,
        None => None,
    };
    scoped.commit().await?;
    Ok(record(p, environment))
}

pub async fn delete(
    tenant_db: &TenantDatabase,
    providers: &S3ProviderClient,
    secrets: &Client,
    organization_id: Uuid,
    actor_id: Uuid,
    project_id: Uuid,
) -> Result<(), AppError> {
    verify_org_owner(tenant_db, organization_id)?;
    let context = ServiceContext { providers, secrets };
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let project = verify_project(tx, organization_id, project_id).await?;
    let access_keys =
        storage_access_tokens::delete_for_project(tx, organization_id, project_id).await?;
    registry_access_tokens::delete_project_in_transaction(tx, organization_id, project_id).await?;
    registry_repositories::delete_project_in_transaction(tx, organization_id, project_id, actor_id)
        .await?;
    storage_buckets::delete_project_in_transaction(tx, organization_id, project_id, actor_id)
        .await?;
    containers::delete_project_in_transaction(tx, organization_id, project_id).await?;
    let deleted = postgres_databases::delete_project_in_transaction(
        tx,
        organization_id,
        actor_id,
        &context,
        project_id,
    )
    .await?;
    project::Entity::delete_by_id(project_id).exec(tx).await?;
    events::record(tx,organization_id,project_id,"project:deleted",serde_json::json!({"summary":format!("Deleted project '{}'",project.name),"target_id":project_id.to_string()}),actor_id).await?;
    scoped.commit().await?;
    providers
        .invalidate_access_token_caches(&access_keys)
        .await
        .map_err(|e| {
            tracing::warn!(%e,%project_id,"project cache invalidation failed after deletion");
            e
        })
        .ok();
    postgres_databases::finalize_deleted_databases(&context, organization_id, &deleted).await?;
    Ok(())
}
