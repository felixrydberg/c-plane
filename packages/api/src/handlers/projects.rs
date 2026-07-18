use axum::{Json, extract::Path};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

use super::databases::verify_org_access;
use crate::errors::AppError;
use crate::middleware::auth::AuthContext;
use crate::models::entities::{
    project, project_branch, project_timeline, stateful_postgres_database,
    stateful_postgres_database_branch,
};
use crate::models::pins::TimelinePins;
use crate::services::agent;
use crate::state::get_app_state;
use crate::utils::pagination::{PaginatedResponse, PaginationQuery};

#[derive(Deserialize, ToSchema)]
pub struct CreateProjectRequest {
    pub name: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateBranchRequest {
    pub name: String,
    pub parent_timeline_id: Option<Uuid>,
    #[serde(default = "default_true")]
    pub auto_branch_databases: bool,
}

pub fn default_true() -> bool {
    true
}

#[derive(Deserialize, ToSchema)]
pub struct ListProjectsQuery {
    pub search: Option<String>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Serialize, ToSchema)]
pub struct ProjectResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub default_branch_id: Option<Uuid>,
    pub main_branch: Option<BranchResponse>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct BranchResponse {
    pub id: Uuid,
    pub name: String,
    pub timeline: String,
    pub is_default: bool,
}

#[derive(Serialize, ToSchema)]
pub struct BranchWithProjectResponse {
    pub id: Uuid,
    pub name: String,
    pub timeline: String,
    pub is_default: bool,
    pub project_id: Uuid,
    pub project_name: String,
}

#[derive(Serialize, ToSchema)]
pub struct TimelineResponse {
    pub id: Uuid,
    pub branch_id: Option<Uuid>,
    pub timeline: i32,
    pub name: Option<String>,
    pub parent_timeline_id: Option<Uuid>,
    pub pins: serde_json::Value,
    pub created_at: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ListTimelinesQuery {
    pub branch_id: Option<Uuid>,
}

async fn branch_databases_for_project(
    tx: &impl sea_orm::ConnectionTrait,
    project_id: Uuid,
    branch_id: Uuid,
    organization_id: Uuid,
) -> Result<(), AppError> {
    let databases = stateful_postgres_database::Entity::find()
        .filter(stateful_postgres_database::Column::ProjectId.eq(project_id))
        .all(tx)
        .await?;

    for db in databases {
        let exists = stateful_postgres_database_branch::Entity::find()
            .filter(stateful_postgres_database_branch::Column::DatabaseId.eq(db.id))
            .filter(stateful_postgres_database_branch::Column::BranchId.eq(branch_id))
            .one(tx)
            .await?
            .is_some();

        if !exists {
            stateful_postgres_database_branch::ActiveModel {
                id: Set(Uuid::new_v4()),
                database_id: Set(db.id),
                branch_id: Set(branch_id),
                organization_id: Set(organization_id),
                cpu: Set(None),
                ram: Set(None),
                high_availability: Set(false),
                read_replicas: Set(None),
                autoscaling_enabled: Set(false),
                autoscaling_min_cpu: Set(None),
                autoscaling_max_cpu: Set(None),
            }
            .insert(tx)
            .await?;
        }
    }

    Ok(())
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/projects",
    request_body = CreateProjectRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
    ),
    responses(
        (status = 201, description = "Project created", body = ProjectResponse),
        (status = 404, description = "Organization not found"),
    ),
    tag = "projects",
)]
pub async fn create_project(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
    Json(body): Json<CreateProjectRequest>,
) -> Result<(axum::http::StatusCode, Json<ProjectResponse>), AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name is required".into()));
    }

    let project_id = Uuid::new_v4();
    let branch_id = Uuid::new_v4();
    let timeline_id = Uuid::new_v4();

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let created: project::Model = project::ActiveModel {
        id: Set(project_id),
        organization_id: Set(organization_id),
        name: Set(name.clone()),
        default_branch_id: Set(None),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;

    let _timeline: project_timeline::Model = project_timeline::ActiveModel {
        id: Set(timeline_id),
        project_id: Set(project_id),
        branch_id: Set(Some(branch_id)),
        organization_id: Set(organization_id),
        timeline: Set(1),
        name: Set(Some("Initial".into())),
        parent_timeline_id: Set(None),
        pins: Set(TimelinePins::default().to_json_value()),
        created_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;

    let main_branch: project_branch::Model = project_branch::ActiveModel {
        id: Set(branch_id),
        project_id: Set(project_id),
        organization_id: Set(organization_id),
        name: Set("main".into()),
        timeline: Set(timeline_id),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;

    let mut project_active: project::ActiveModel = created.clone().into();
    project_active.default_branch_id = Set(Some(branch_id));
    project_active.updated_at = Set(Utc::now().fixed_offset());
    let updated_project: project::Model = project_active.update(tx).await?;

    scoped.commit().await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(ProjectResponse {
            id: updated_project.id,
            organization_id: updated_project.organization_id,
            name: updated_project.name,
            default_branch_id: updated_project.default_branch_id,
            main_branch: Some(BranchResponse {
                id: main_branch.id,
                name: main_branch.name,
                timeline: main_branch.timeline.to_string(),
                is_default: true,
            }),
            created_at: updated_project.created_at.to_string(),
            updated_at: updated_project.updated_at.to_string(),
        }),
    ))
}

pub async fn list_projects(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
    axum::extract::Query(query): axum::extract::Query<ListProjectsQuery>,
) -> Result<Json<PaginatedResponse<ProjectResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let pagination = PaginationQuery {
        page: query.page,
        per_page: query.per_page,
    };
    let page = pagination.page();
    let per_page = pagination.per_page();

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    use project::{Column, Entity};
    let mut select = Entity::find().filter(Column::OrganizationId.eq(organization_id));

    if let Some(ref search) = query.search
        && !search.trim().is_empty()
    {
        select = select.filter(Column::Name.contains(search.trim()));
    }

    let total = select.clone().count(tx).await?;

    let projects_with_branches: Vec<(project::Model, Option<project_branch::Model>)> = select
        .order_by_asc(Column::Name)
        .find_also_related(project_branch::Entity)
        .paginate(tx, per_page)
        .fetch_page(page - 1)
        .await?;

    scoped.commit().await?;

    let data = projects_with_branches
        .into_iter()
        .map(|(p, branch)| ProjectResponse {
            id: p.id,
            organization_id: p.organization_id,
            name: p.name,
            default_branch_id: p.default_branch_id,
            main_branch: branch.map(|b| BranchResponse {
                id: b.id,
                name: b.name,
                timeline: b.timeline.to_string(),
                is_default: p.default_branch_id == Some(b.id),
            }),
            created_at: p.created_at.to_string(),
            updated_at: p.updated_at.to_string(),
        })
        .collect();

    Ok(Json(PaginatedResponse::new(data, total, page, per_page)))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/projects/{project_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
    ),
    responses(
        (status = 200, description = "Project details", body = ProjectResponse),
        (status = 404, description = "Not found"),
    ),
    tag = "projects",
)]
pub async fn get_project(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ProjectResponse>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    use project::{Column, Entity};
    let p = Entity::find()
        .filter(Column::Id.eq(project_id))
        .filter(Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".into()))?;

    let main_branch = if let Some(branch_id) = p.default_branch_id {
        project_branch::Entity::find_by_id(branch_id)
            .one(tx)
            .await?
            .map(|b| BranchResponse {
                id: b.id,
                name: b.name,
                timeline: b.timeline.to_string(),
                is_default: p.default_branch_id == Some(b.id),
            })
    } else {
        None
    };

    scoped.commit().await?;

    Ok(Json(ProjectResponse {
        id: p.id,
        organization_id: p.organization_id,
        name: p.name,
        default_branch_id: p.default_branch_id,
        main_branch,
        created_at: p.created_at.to_string(),
        updated_at: p.updated_at.to_string(),
    }))
}

#[utoipa::path(
    delete,
    path = "/api/organization/{organization_id}/projects/{project_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
    ),
    responses(
        (status = 200, description = "Project deleted"),
        (status = 404, description = "Not found"),
    ),
    tag = "projects",
)]
pub async fn delete_project(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    use project::{Column, Entity};
    let exists = Entity::find()
        .filter(Column::Id.eq(project_id))
        .filter(Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .is_some();

    if !exists {
        return Err(AppError::NotFound("Project not found".into()));
    }

    Entity::delete_by_id(project_id).exec(tx).await?;
    scoped.commit().await?;
    if let Some(s3_providers) = get_app_state().s3_providers {
        s3_providers.invalidate_access_token_cache().await?;
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn list_organization_branches(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
) -> Result<Json<Vec<BranchWithProjectResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let branches = project_branch::Entity::find()
        .filter(project_branch::Column::OrganizationId.eq(organization_id))
        .order_by_asc(project_branch::Column::Name)
        .all(tx)
        .await?;

    let project_ids: Vec<Uuid> = branches.iter().map(|b| b.project_id).collect();

    let projects = project::Entity::find()
        .filter(project::Column::Id.is_in(project_ids))
        .all(tx)
        .await?;

    let mut project_names: HashMap<Uuid, String> = HashMap::new();
    let mut project_defaults: HashMap<Uuid, Option<Uuid>> = HashMap::new();
    for p in projects {
        let project_id = p.id;
        let default_branch_id = p.default_branch_id;
        project_names.insert(project_id, p.name);
        project_defaults.insert(project_id, default_branch_id);
    }

    scoped.commit().await?;

    let responses = branches
        .into_iter()
        .map(|b| {
            let is_default = project_defaults
                .get(&b.project_id)
                .map(|default_id| *default_id == Some(b.id))
                .unwrap_or(false);
            BranchWithProjectResponse {
                id: b.id,
                name: b.name,
                timeline: b.timeline.to_string(),
                is_default,
                project_id: b.project_id,
                project_name: project_names
                    .get(&b.project_id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".into()),
            }
        })
        .collect();

    Ok(Json(responses))
}

pub async fn list_branches(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<BranchResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let project = project::Entity::find()
        .filter(project::Column::Id.eq(project_id))
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".into()))?;

    use project_branch::{Column, Entity};
    let branches = Entity::find()
        .filter(Column::ProjectId.eq(project_id))
        .order_by_asc(Column::Name)
        .all(tx)
        .await?;

    scoped.commit().await?;

    let responses = branches
        .into_iter()
        .map(|b| BranchResponse {
            id: b.id,
            name: b.name,
            timeline: b.timeline.to_string(),
            is_default: project.default_branch_id == Some(b.id),
        })
        .collect();

    Ok(Json(responses))
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/projects/{project_id}/branches",
    request_body = CreateBranchRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
    ),
    responses(
        (status = 201, description = "Branch created", body = BranchResponse),
        (status = 404, description = "Not found"),
        (status = 409, description = "Branch name already exists"),
    ),
    tag = "branches",
)]
pub async fn create_branch(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateBranchRequest>,
) -> Result<(axum::http::StatusCode, Json<BranchResponse>), AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name is required".into()));
    }

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let project = project::Entity::find()
        .filter(project::Column::Id.eq(project_id))
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".into()))?;

    let existing = project_branch::Entity::find()
        .filter(project_branch::Column::ProjectId.eq(project_id))
        .filter(project_branch::Column::Name.eq(&name))
        .one(tx)
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(
            "A branch with this name already exists".into(),
        ));
    }

    let pins = if let Some(parent_timeline_id) = body.parent_timeline_id {
        let parent = project_timeline::Entity::find()
            .filter(project_timeline::Column::Id.eq(parent_timeline_id))
            .filter(project_timeline::Column::ProjectId.eq(project_id))
            .one(tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Timeline revision not found".into()))?;
        parent.pins.clone()
    } else {
        serde_json::json!({ "container": {}, "secret": {} })
    };

    let branch_id = Uuid::new_v4();
    let timeline_id = Uuid::new_v4();

    let parent_timeline_id = body.parent_timeline_id;

    let _timeline: project_timeline::Model = project_timeline::ActiveModel {
        id: Set(timeline_id),
        project_id: Set(project_id),
        branch_id: Set(Some(branch_id)),
        organization_id: Set(organization_id),
        timeline: Set(1),
        name: Set(Some(format!("Branch '{}' created", name))),
        parent_timeline_id: Set(parent_timeline_id),
        pins: Set(pins),
        created_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;

    let branch: project_branch::Model = project_branch::ActiveModel {
        id: Set(branch_id),
        project_id: Set(project_id),
        organization_id: Set(organization_id),
        name: Set(name),
        timeline: Set(timeline_id),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;

    if body.auto_branch_databases {
        branch_databases_for_project(tx, project_id, branch_id, organization_id).await?;
    }

    scoped.commit().await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(BranchResponse {
            id: branch.id,
            name: branch.name,
            timeline: branch.timeline.to_string(),
            is_default: project.default_branch_id == Some(branch.id),
        }),
    ))
}

pub async fn list_project_timelines(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
    axum::extract::Query(query): axum::extract::Query<ListTimelinesQuery>,
) -> Result<Json<Vec<TimelineResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let exists = project::Entity::find()
        .filter(project::Column::Id.eq(project_id))
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .is_some();

    if !exists {
        return Err(AppError::NotFound("Project not found".into()));
    }

    use project_timeline::{Column, Entity};
    let mut select = Entity::find().filter(Column::ProjectId.eq(project_id));

    if let Some(branch_id) = query.branch_id {
        select = select.filter(Column::BranchId.eq(branch_id));
    }

    let timelines = select
        .order_by_desc(Column::Timeline)
        .order_by_desc(Column::CreatedAt)
        .all(tx)
        .await?;

    scoped.commit().await?;

    let responses = timelines
        .into_iter()
        .map(|t| TimelineResponse {
            id: t.id,
            branch_id: t.branch_id,
            timeline: t.timeline,
            name: t.name,
            parent_timeline_id: t.parent_timeline_id,
            pins: t.pins,
            created_at: t.created_at.to_string(),
        })
        .collect();

    Ok(Json(responses))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateBranchRequest {
    pub timeline_id: Uuid,
    #[serde(default)]
    pub branch_databases: bool,
}

#[utoipa::path(
    patch,
    path = "/api/organization/{organization_id}/projects/{project_id}/branches/{branch_id}",
    request_body = UpdateBranchRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("branch_id" = Uuid, Path, description = "Branch ID"),
    ),
    responses(
        (status = 200, description = "Branch updated", body = BranchResponse),
        (status = 404, description = "Not found"),
    ),
    tag = "branches",
)]
pub async fn update_branch(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id, branch_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<UpdateBranchRequest>,
) -> Result<Json<BranchResponse>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let project = project::Entity::find()
        .filter(project::Column::Id.eq(project_id))
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".into()))?;

    let _timeline = project_timeline::Entity::find()
        .filter(project_timeline::Column::Id.eq(body.timeline_id))
        .filter(project_timeline::Column::ProjectId.eq(project_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Timeline revision not found".into()))?;

    let mut active: project_branch::ActiveModel = project_branch::Entity::find_by_id(branch_id)
        .filter(project_branch::Column::ProjectId.eq(project_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Branch not found".into()))?
        .into();

    active.timeline = Set(body.timeline_id);
    active.updated_at = Set(Utc::now().fixed_offset());
    let updated = active.update(tx).await?;

    if body.branch_databases {
        branch_databases_for_project(tx, project_id, branch_id, organization_id).await?;
    }

    agent::emit_project(project.id, organization_id, branch_id, body.timeline_id).await?;

    scoped.commit().await?;

    Ok(Json(BranchResponse {
        id: updated.id,
        name: updated.name,
        timeline: updated.timeline.to_string(),
        is_default: project.default_branch_id == Some(updated.id),
    }))
}

#[utoipa::path(
    delete,
    path = "/api/organization/{organization_id}/projects/{project_id}/branches/{branch_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("branch_id" = Uuid, Path, description = "Branch ID"),
    ),
    responses(
        (status = 200, description = "Branch deleted"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Cannot delete default branch"),
    ),
    tag = "branches",
)]
pub async fn delete_branch(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id, branch_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let project = project::Entity::find()
        .filter(project::Column::Id.eq(project_id))
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".into()))?;

    if project.default_branch_id == Some(branch_id) {
        return Err(AppError::Conflict(
            "Cannot delete the default branch".into(),
        ));
    }

    let branch = project_branch::Entity::find_by_id(branch_id)
        .filter(project_branch::Column::ProjectId.eq(project_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Branch not found".into()))?;

    // Delete timeline revisions belonging to this branch
    let deleted_timeline_ids: Vec<Uuid> = project_timeline::Entity::find()
        .filter(project_timeline::Column::ProjectId.eq(project_id))
        .filter(project_timeline::Column::BranchId.eq(branch_id))
        .all(tx)
        .await?
        .into_iter()
        .map(|t| t.id)
        .collect();

    if !deleted_timeline_ids.is_empty() {
        // Clear parent references on other revisions that pointed to deleted timelines
        let mut other_revisions = project_timeline::Entity::find()
            .filter(project_timeline::Column::ProjectId.eq(project_id))
            .filter(project_timeline::Column::ParentTimelineId.is_in(deleted_timeline_ids.clone()))
            .all(tx)
            .await?;

        for rev in &mut other_revisions {
            let mut active: project_timeline::ActiveModel = rev.clone().into();
            active.parent_timeline_id = Set(None);
            active.update(tx).await?;
        }
    }

    project_branch::Entity::delete_by_id(branch.id)
        .exec(tx)
        .await?;

    if !deleted_timeline_ids.is_empty() {
        project_timeline::Entity::delete_many()
            .filter(project_timeline::Column::Id.is_in(deleted_timeline_ids))
            .exec(tx)
            .await?;
    }

    scoped.commit().await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(Serialize, ToSchema)]
pub struct ResolvedContainerPin {
    pub container_id: Uuid,
    pub container_name: String,
    pub version_id: Uuid,
    pub version: i32,
    pub image: String,
}

#[derive(Serialize, ToSchema)]
pub struct ResolvedTimelineResponse {
    pub id: Uuid,
    pub branch_id: Option<Uuid>,
    pub timeline: i32,
    pub name: Option<String>,
    pub parent_timeline_id: Option<Uuid>,
    pub containers: Vec<ResolvedContainerPin>,
    pub created_at: String,
}

pub async fn get_timeline(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id, timeline_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<ResolvedTimelineResponse>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let t = project_timeline::Entity::find()
        .filter(project_timeline::Column::Id.eq(timeline_id))
        .filter(project_timeline::Column::ProjectId.eq(project_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Timeline not found".into()))?;

    let pins = TimelinePins::from_json_value(&t.pins);

    let mut containers = Vec::new();

    if !pins.container.is_empty() {
        let container_ids: Vec<Uuid> = pins.container.keys().cloned().collect();
        let version_ids: Vec<Uuid> = pins.container.values().cloned().collect();

        let container_models = crate::models::entities::container::Entity::find()
            .filter(crate::models::entities::container::Column::Id.is_in(container_ids))
            .all(tx)
            .await?;

        let container_names: HashMap<Uuid, String> = container_models
            .into_iter()
            .map(|c| (c.id, c.name))
            .collect();

        let version_models = crate::models::entities::container_version::Entity::find()
            .filter(crate::models::entities::container_version::Column::Id.is_in(version_ids))
            .all(tx)
            .await?;

        let version_map: HashMap<Uuid, &crate::models::entities::container_version::Model> =
            version_models.iter().map(|v| (v.id, v)).collect();

        for (container_id, version_id) in &pins.container {
            if let Some(version) = version_map.get(version_id) {
                containers.push(ResolvedContainerPin {
                    container_id: *container_id,
                    container_name: container_names
                        .get(container_id)
                        .cloned()
                        .unwrap_or_else(|| "Unknown".into()),
                    version_id: *version_id,
                    version: version.version,
                    image: version.image.clone(),
                });
            }
        }
    }

    scoped.commit().await?;

    Ok(Json(ResolvedTimelineResponse {
        id: t.id,
        branch_id: t.branch_id,
        timeline: t.timeline,
        name: t.name,
        parent_timeline_id: t.parent_timeline_id,
        containers,
        created_at: t.created_at.to_string(),
    }))
}
