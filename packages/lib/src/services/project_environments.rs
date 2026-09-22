use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::{
    entities::{project, project_environment, project_timeline},
    error::AppError,
    manifest::RevisionManifest,
    services::{agent, events, revisions},
    tenant::TenantDatabase,
};

pub struct CreatedEnvironment {
    pub environment: project_environment::Model,
    pub is_default: bool,
}
pub struct ListedEnvironment {
    pub environment: project_environment::Model,
    pub project_name: Option<String>,
    pub is_default: bool,
}
pub struct UpdatedEnvironment {
    pub environment: project_environment::Model,
    pub is_default: bool,
}

pub async fn create_initial_in_transaction(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
    environment_id: Uuid,
) -> Result<project_environment::Model, AppError> {
    let timeline = revisions::insert_revision(
        tx,
        project_id,
        organization_id,
        Some(environment_id),
        None,
        &RevisionManifest::default(),
        Some("Initial".into()),
    )
    .await?;
    Ok(project_environment::ActiveModel {
        id: Set(environment_id),
        project_id: Set(project_id),
        organization_id: Set(organization_id),
        name: Set("main".into()),
        is_preview: Set(false),
        draft_timeline: Set(timeline.id),
        deployed_timeline: Set(timeline.id),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
        ..Default::default()
    }
    .insert(tx)
    .await?)
}

fn access(tenant_db: &TenantDatabase, organization_id: Uuid) -> Result<(), AppError> {
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
fn owner(tenant_db: &TenantDatabase, organization_id: Uuid) -> Result<(), AppError> {
    access(tenant_db, organization_id)?;
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
async fn project(
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
    project_id: Uuid,
    actor_id: Uuid,
    name: &str,
    parent_timeline_id: Option<Uuid>,
    is_preview: bool,
) -> Result<CreatedEnvironment, AppError> {
    access(tenant_db, organization_id)?;
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name is required".into()));
    }
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let project = project(tx, organization_id, project_id).await?;
    if project_environment::Entity::find()
        .filter(project_environment::Column::ProjectId.eq(project_id))
        .filter(project_environment::Column::Name.eq(name))
        .one(tx)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(
            "A environment with this name already exists".into(),
        ));
    }
    let environment_id = Uuid::new_v4();
    let timeline_id = if let Some(parent) = parent_timeline_id {
        let row = project_timeline::Entity::find_by_id(parent)
            .filter(project_timeline::Column::ProjectId.eq(project_id))
            .one(tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Timeline revision not found".into()))?;
        revisions::load_manifest(tx, row.manifest_id).await?;
        row.id
    } else {
        revisions::insert_revision(
            tx,
            project_id,
            organization_id,
            Some(environment_id),
            None,
            &RevisionManifest::default(),
            Some(format!("Environment '{}' created", name)),
        )
        .await?
        .id
    };
    let environment = project_environment::ActiveModel {
        id: Set(environment_id),
        project_id: Set(project_id),
        organization_id: Set(organization_id),
        name: Set(name.into()),
        is_preview: Set(is_preview),
        draft_timeline: Set(timeline_id),
        deployed_timeline: Set(timeline_id),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    let is_default = project.default_environment_id == Some(environment.id);
    events::record(tx, organization_id, project_id, "environment:created", serde_json::json!({"summary": format!("Created environment '{}'", environment.name), "target_id": environment.id.to_string()}), actor_id).await?;
    scoped.commit().await?;
    Ok(CreatedEnvironment {
        environment,
        is_default,
    })
}

pub async fn delete(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
    environment_id: Uuid,
    actor_id: Uuid,
) -> Result<(), AppError> {
    owner(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let project = project(tx, organization_id, project_id).await?;
    if project.default_environment_id == Some(environment_id) {
        return Err(AppError::Conflict(
            "Cannot delete the default environment".into(),
        ));
    }
    let environment = project_environment::Entity::find_by_id(environment_id)
        .filter(project_environment::Column::ProjectId.eq(project_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Environment not found".into()))?;
    revisions::lock_project(tx, project_id).await?;
    project_timeline::Entity::update_many()
        .col_expr(
            project_timeline::Column::EnvironmentId,
            sea_orm::sea_query::Expr::value(Option::<Uuid>::None),
        )
        .filter(project_timeline::Column::ProjectId.eq(project_id))
        .filter(project_timeline::Column::EnvironmentId.eq(environment_id))
        .exec(tx)
        .await?;
    project_environment::Entity::delete_by_id(environment.id)
        .exec(tx)
        .await?;
    events::record(tx, organization_id, project_id, "environment:deleted", serde_json::json!({"summary": format!("Deleted environment '{}'", environment.name), "target_id": environment.id.to_string()}), actor_id).await?;
    scoped.commit().await?;
    Ok(())
}

pub async fn list_organization(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
) -> Result<Vec<ListedEnvironment>, AppError> {
    access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let rows = project_environment::Entity::find()
        .filter(project_environment::Column::OrganizationId.eq(organization_id))
        .order_by_asc(project_environment::Column::Name)
        .find_also_related(project::Entity)
        .all(scoped.connection())
        .await?;
    scoped.commit().await?;
    Ok(rows
        .into_iter()
        .map(|(environment, project)| ListedEnvironment {
            is_default: project
                .as_ref()
                .is_some_and(|p| p.default_environment_id == Some(environment.id)),
            project_name: project.map(|p| p.name),
            environment,
        })
        .collect())
}

pub async fn list(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
) -> Result<Vec<ListedEnvironment>, AppError> {
    access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let project = project(scoped.connection(), organization_id, project_id).await?;
    let rows = project_environment::Entity::find()
        .filter(project_environment::Column::ProjectId.eq(project_id))
        .order_by_asc(project_environment::Column::Name)
        .all(scoped.connection())
        .await?;
    scoped.commit().await?;
    Ok(rows
        .into_iter()
        .map(|environment| ListedEnvironment {
            is_default: project.default_environment_id == Some(environment.id),
            project_name: None,
            environment,
        })
        .collect())
}

pub async fn update(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
    environment_id: Uuid,
    actor_id: Uuid,
    name: Option<String>,
    draft_timeline_id: Option<Uuid>,
    deployed_timeline_id: Option<Uuid>,
) -> Result<UpdatedEnvironment, AppError> {
    access(tenant_db, organization_id)?;
    if name.as_ref().is_some_and(String::is_empty) {
        return Err(AppError::BadRequest("Name is required".into()));
    }
    if name.is_none() && draft_timeline_id.is_none() && deployed_timeline_id.is_none() {
        return Err(AppError::BadRequest("Name or revision is required".into()));
    }
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let project = project(tx, organization_id, project_id).await?;
    revisions::lock_project(tx, project_id).await?;
    for timeline_id in [draft_timeline_id, deployed_timeline_id]
        .into_iter()
        .flatten()
    {
        let timeline = project_timeline::Entity::find_by_id(timeline_id)
            .filter(project_timeline::Column::ProjectId.eq(project_id))
            .one(tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Timeline revision not found".into()))?;
        revisions::load_manifest(tx, timeline.manifest_id).await?;
    }
    let environment = project_environment::Entity::find_by_id(environment_id)
        .filter(project_environment::Column::ProjectId.eq(project_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Environment not found".into()))?;
    if let Some(ref name) = name {
        if name != &environment.name
            && project_environment::Entity::find()
                .filter(project_environment::Column::ProjectId.eq(project_id))
                .filter(project_environment::Column::Name.eq(name))
                .one(tx)
                .await?
                .is_some()
        {
            return Err(AppError::Conflict(
                "A environment with this name already exists".into(),
            ));
        }
    }
    let mut active: project_environment::ActiveModel = environment.into();
    if let Some(name) = name {
        active.name = Set(name);
    }
    if let Some(id) = draft_timeline_id {
        active.draft_timeline = Set(id);
    }
    if let Some(id) = deployed_timeline_id {
        active.deployed_timeline = Set(id);
    }
    active.updated_at = Set(Utc::now().fixed_offset());
    let updated = active.update(tx).await?;
    events::record(tx,organization_id,project_id,"environment:updated",serde_json::json!({"summary":format!("Updated environment '{}'",updated.name),"target_id":updated.id.to_string()}),actor_id).await?;
    scoped.commit().await?;
    if let Some(timeline_id) = deployed_timeline_id {
        agent::emit_compute(project_id, organization_id, environment_id, timeline_id).await?;
    }
    Ok(UpdatedEnvironment {
        is_default: project.default_environment_id == Some(updated.id),
        environment: updated,
    })
}
