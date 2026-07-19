use axum::{Json, extract::Path};
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use utoipa::ToSchema;
use uuid::Uuid;

use super::databases::verify_org_access;
use crate::errors::AppError;
use crate::middleware::auth::AuthContext;
use crate::models::entities::{project, project_environment, project_timeline};
use crate::models::pins::TimelinePins;
use crate::services::agent;
use crate::state::get_app_state;
use crate::utils::pagination::{PaginatedResponse, PaginationQuery};

#[derive(Deserialize, ToSchema)]
pub struct CreateProjectRequest {
    pub name: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateEnvironmentRequest {
    pub name: String,
    pub parent_timeline_id: Option<Uuid>,
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
    pub default_environment_id: Option<Uuid>,
    pub main_environment: Option<EnvironmentResponse>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct EnvironmentResponse {
    pub id: Uuid,
    pub name: String,
    pub timeline: String,
    pub is_default: bool,
    pub has_recent_undeployed_revision: bool,
}

#[derive(Serialize, ToSchema)]
pub struct EnvironmentWithProjectResponse {
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
    pub environment_id: Option<Uuid>,
    pub timeline: i32,
    pub name: Option<String>,
    pub parent_timeline_id: Option<Uuid>,
    pub pins: serde_json::Value,
    pub created_at: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ListTimelinesQuery {
    pub environment_id: Option<Uuid>,
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
    let environment_id = Uuid::new_v4();
    let timeline_id = Uuid::new_v4();

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let created: project::Model = project::ActiveModel {
        id: Set(project_id),
        organization_id: Set(organization_id),
        name: Set(name.clone()),
        default_environment_id: Set(None),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;

    let _timeline: project_timeline::Model = project_timeline::ActiveModel {
        id: Set(timeline_id),
        project_id: Set(project_id),
        environment_id: Set(Some(environment_id)),
        organization_id: Set(organization_id),
        timeline: Set(1),
        name: Set(Some("Initial".into())),
        parent_timeline_id: Set(None),
        pins: Set(TimelinePins::default().to_json_value()),
        created_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;

    let main_environment: project_environment::Model = project_environment::ActiveModel {
        id: Set(environment_id),
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
    project_active.default_environment_id = Set(Some(environment_id));
    project_active.updated_at = Set(Utc::now().fixed_offset());
    let updated_project: project::Model = project_active.update(tx).await?;

    scoped.commit().await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(ProjectResponse {
            id: updated_project.id,
            organization_id: updated_project.organization_id,
            name: updated_project.name,
            default_environment_id: updated_project.default_environment_id,
            main_environment: Some(EnvironmentResponse {
                id: main_environment.id,
                name: main_environment.name,
                timeline: main_environment.timeline.to_string(),
                is_default: true,
                has_recent_undeployed_revision: false,
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

    let projects_with_environments: Vec<(project::Model, Option<project_environment::Model>)> =
        select
            .order_by_asc(Column::Name)
            .find_also_related(project_environment::Entity)
            .paginate(tx, per_page)
            .fetch_page(page - 1)
            .await?;

    scoped.commit().await?;

    let data = projects_with_environments
        .into_iter()
        .map(|(p, environment)| ProjectResponse {
            id: p.id,
            organization_id: p.organization_id,
            name: p.name,
            default_environment_id: p.default_environment_id,
            main_environment: environment.map(|b| EnvironmentResponse {
                id: b.id,
                name: b.name,
                timeline: b.timeline.to_string(),
                is_default: p.default_environment_id == Some(b.id),
                has_recent_undeployed_revision: false,
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

    let main_environment = if let Some(environment_id) = p.default_environment_id {
        project_environment::Entity::find_by_id(environment_id)
            .one(tx)
            .await?
            .map(|b| EnvironmentResponse {
                id: b.id,
                name: b.name,
                timeline: b.timeline.to_string(),
                is_default: p.default_environment_id == Some(b.id),
                has_recent_undeployed_revision: false,
            })
    } else {
        None
    };

    scoped.commit().await?;

    Ok(Json(ProjectResponse {
        id: p.id,
        organization_id: p.organization_id,
        name: p.name,
        default_environment_id: p.default_environment_id,
        main_environment,
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

pub async fn list_organization_environments(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
) -> Result<Json<Vec<EnvironmentWithProjectResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let environments = project_environment::Entity::find()
        .filter(project_environment::Column::OrganizationId.eq(organization_id))
        .order_by_asc(project_environment::Column::Name)
        .all(tx)
        .await?;

    let project_ids: Vec<Uuid> = environments.iter().map(|b| b.project_id).collect();

    let projects = project::Entity::find()
        .filter(project::Column::Id.is_in(project_ids))
        .all(tx)
        .await?;

    let mut project_names: HashMap<Uuid, String> = HashMap::new();
    let mut project_defaults: HashMap<Uuid, Option<Uuid>> = HashMap::new();
    for p in projects {
        let project_id = p.id;
        let default_environment_id = p.default_environment_id;
        project_names.insert(project_id, p.name);
        project_defaults.insert(project_id, default_environment_id);
    }

    scoped.commit().await?;

    let responses = environments
        .into_iter()
        .map(|b| {
            let is_default = project_defaults
                .get(&b.project_id)
                .map(|default_id| *default_id == Some(b.id))
                .unwrap_or(false);
            EnvironmentWithProjectResponse {
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

pub async fn list_environments(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<EnvironmentResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let project = project::Entity::find()
        .filter(project::Column::Id.eq(project_id))
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".into()))?;

    use project_environment::{Column, Entity};
    let environments = Entity::find()
        .filter(Column::ProjectId.eq(project_id))
        .order_by_asc(Column::Name)
        .all(tx)
        .await?;

    let environment_updated_at: HashMap<Uuid, _> = environments
        .iter()
        .map(|environment| (environment.id, environment.updated_at))
        .collect();
    let undeployed_environment_ids: HashSet<Uuid> = project_timeline::Entity::find()
        .filter(project_timeline::Column::ProjectId.eq(project_id))
        .filter(
            project_timeline::Column::CreatedAt.gte(Utc::now().fixed_offset() - Duration::hours(1)),
        )
        .all(tx)
        .await?
        .into_iter()
        .filter_map(|revision| {
            let environment_id = revision.environment_id?;
            environment_updated_at
                .get(&environment_id)
                .is_some_and(|updated_at| revision.created_at > *updated_at)
                .then_some(environment_id)
        })
        .collect();

    scoped.commit().await?;

    let responses = environments
        .into_iter()
        .map(|b| EnvironmentResponse {
            id: b.id,
            name: b.name,
            timeline: b.timeline.to_string(),
            is_default: project.default_environment_id == Some(b.id),
            has_recent_undeployed_revision: undeployed_environment_ids.contains(&b.id),
        })
        .collect();

    Ok(Json(responses))
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/projects/{project_id}/environments",
    request_body = CreateEnvironmentRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
    ),
    responses(
        (status = 201, description = "Environment created", body = EnvironmentResponse),
        (status = 404, description = "Not found"),
        (status = 409, description = "Environment name already exists"),
    ),
    tag = "environments",
)]
pub async fn create_environment(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateEnvironmentRequest>,
) -> Result<(axum::http::StatusCode, Json<EnvironmentResponse>), AppError> {
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

    let existing = project_environment::Entity::find()
        .filter(project_environment::Column::ProjectId.eq(project_id))
        .filter(project_environment::Column::Name.eq(&name))
        .one(tx)
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(
            "A environment with this name already exists".into(),
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

    let environment_id = Uuid::new_v4();
    let timeline_id = Uuid::new_v4();

    let parent_timeline_id = body.parent_timeline_id;

    let _timeline: project_timeline::Model = project_timeline::ActiveModel {
        id: Set(timeline_id),
        project_id: Set(project_id),
        environment_id: Set(Some(environment_id)),
        organization_id: Set(organization_id),
        timeline: Set(1),
        name: Set(Some(format!("Environment '{}' created", name))),
        parent_timeline_id: Set(parent_timeline_id),
        pins: Set(pins),
        created_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;

    let environment: project_environment::Model = project_environment::ActiveModel {
        id: Set(environment_id),
        project_id: Set(project_id),
        organization_id: Set(organization_id),
        name: Set(name),
        timeline: Set(timeline_id),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;

    scoped.commit().await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(EnvironmentResponse {
            id: environment.id,
            name: environment.name,
            timeline: environment.timeline.to_string(),
            is_default: project.default_environment_id == Some(environment.id),
            has_recent_undeployed_revision: false,
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

    if let Some(environment_id) = query.environment_id {
        select = select.filter(Column::EnvironmentId.eq(environment_id));
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
            environment_id: t.environment_id,
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
pub struct UpdateEnvironmentRequest {
    pub name: Option<String>,
    pub timeline_id: Option<Uuid>,
}

#[cfg(test)]
mod update_environment_request_tests {
    use super::UpdateEnvironmentRequest;

    #[test]
    fn accepts_a_rename_without_a_timeline_change() {
        let request: UpdateEnvironmentRequest =
            serde_json::from_str(r#"{"name":"staging"}"#).unwrap();

        assert_eq!(request.name.as_deref(), Some("staging"));
        assert_eq!(request.timeline_id, None);
    }
}

#[utoipa::path(
    patch,
    path = "/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}",
    request_body = UpdateEnvironmentRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("environment_id" = Uuid, Path, description = "Environment ID"),
    ),
    responses(
        (status = 200, description = "Environment updated", body = EnvironmentResponse),
        (status = 400, description = "Name or timeline is required"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Environment name already exists"),
    ),
    tag = "environments",
)]
pub async fn update_environment(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id, environment_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<UpdateEnvironmentRequest>,
) -> Result<Json<EnvironmentResponse>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let name = body.name.map(|name| name.trim().to_string());
    if name.as_ref().is_some_and(String::is_empty) {
        return Err(AppError::BadRequest("Name is required".into()));
    }
    if name.is_none() && body.timeline_id.is_none() {
        return Err(AppError::BadRequest("Name or timeline is required".into()));
    }

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let project = project::Entity::find()
        .filter(project::Column::Id.eq(project_id))
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".into()))?;

    if let Some(timeline_id) = body.timeline_id {
        project_timeline::Entity::find()
            .filter(project_timeline::Column::Id.eq(timeline_id))
            .filter(project_timeline::Column::ProjectId.eq(project_id))
            .one(tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Timeline revision not found".into()))?;
    }

    let environment = project_environment::Entity::find_by_id(environment_id)
        .filter(project_environment::Column::ProjectId.eq(project_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Environment not found".into()))?;

    if let Some(ref name) = name
        && name != &environment.name
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

    let mut active: project_environment::ActiveModel = environment.into();
    if let Some(name) = name {
        active.name = Set(name);
    }
    if let Some(timeline_id) = body.timeline_id {
        active.timeline = Set(timeline_id);
    }
    active.updated_at = Set(Utc::now().fixed_offset());
    let updated = active.update(tx).await?;

    scoped.commit().await?;
    if let Some(timeline_id) = body.timeline_id {
        agent::emit_compute(project.id, organization_id, environment_id, timeline_id).await?;
    }

    Ok(Json(EnvironmentResponse {
        id: updated.id,
        name: updated.name,
        timeline: updated.timeline.to_string(),
        is_default: project.default_environment_id == Some(updated.id),
        has_recent_undeployed_revision: false,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("environment_id" = Uuid, Path, description = "Environment ID"),
    ),
    responses(
        (status = 200, description = "Environment deleted"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Cannot delete default environment"),
    ),
    tag = "environments",
)]
pub async fn delete_environment(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id, environment_id)): Path<(Uuid, Uuid, Uuid)>,
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

    // Delete timeline revisions belonging to this environment
    let deleted_timeline_ids: Vec<Uuid> = project_timeline::Entity::find()
        .filter(project_timeline::Column::ProjectId.eq(project_id))
        .filter(project_timeline::Column::EnvironmentId.eq(environment_id))
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

    project_environment::Entity::delete_by_id(environment.id)
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
    pub environment_id: Option<Uuid>,
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
        environment_id: t.environment_id,
        timeline: t.timeline,
        name: t.name,
        parent_timeline_id: t.parent_timeline_id,
        containers,
        created_at: t.created_at.to_string(),
    }))
}
