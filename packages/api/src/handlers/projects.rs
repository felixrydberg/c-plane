use axum::{Json, extract::Path};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::auth::AuthContext;
use crate::state::get_app_state;
use crate::utils::pagination::{PaginatedResponse, PaginationQuery};
use lib::services::{project_environments, project_timelines, projects as project_service};

#[derive(Deserialize, ToSchema)]
pub struct CreateProjectRequest {
    pub name: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateEnvironmentRequest {
    pub name: String,
    pub parent_timeline_id: Option<Uuid>,
    #[serde(default = "default_preview")]
    pub is_preview: bool,
}

fn default_preview() -> bool {
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
    pub default_environment_id: Option<Uuid>,
    pub main_environment: Option<EnvironmentResponse>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct EnvironmentResponse {
    pub id: Uuid,
    pub name: String,
    pub is_preview: bool,
    pub draft_timeline: String,
    pub deployed_timeline: String,
    pub is_default: bool,
}

#[derive(Serialize, ToSchema)]
pub struct EnvironmentWithProjectResponse {
    pub id: Uuid,
    pub name: String,
    pub is_preview: bool,
    pub draft_timeline: String,
    pub deployed_timeline: String,
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
    pub created_at: String,
}

#[derive(Serialize, ToSchema, Default)]
pub struct TimelinePageResponse {
    pub data: Vec<TimelineResponse>,
    pub next_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_pages: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_newer: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_older: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_nodes: Option<Vec<TimelineGraphNode>>,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct TimelineGraphNode {
    pub id: Uuid,
    pub timeline: i32,
    pub parent_id: Option<Uuid>,
    pub lane: usize,
    pub child_ids: Vec<Uuid>,
}

#[derive(Deserialize, ToSchema)]
pub struct ListTimelinesQuery {
    pub environment_id: Option<Uuid>,
    pub limit: Option<u64>,
    pub cursor: Option<String>,
    pub anchor_revision_id: Option<Uuid>,
    pub graph: Option<bool>,
    pub page: Option<u64>,
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
    AuthContext { tenant_db, auth }: AuthContext,
    Path(organization_id): Path<Uuid>,
    Json(body): Json<CreateProjectRequest>,
) -> Result<(axum::http::StatusCode, Json<ProjectResponse>), AppError> {
    let project =
        project_service::create(&tenant_db, organization_id, auth.actor_id, &body.name).await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(project_response(project)),
    ))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/projects",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("search" = Option<String>, Query, description = "Project name search"),
        ("page" = Option<u64>, Query, description = "Page number"),
        ("per_page" = Option<u64>, Query, description = "Items per page"),
    ),
    responses((status = 200, description = "Paginated projects", body = PaginatedResponse<ProjectResponse>)),
    tag = "projects",
)]
pub async fn list_projects(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
    axum::extract::Query(query): axum::extract::Query<ListProjectsQuery>,
) -> Result<Json<PaginatedResponse<ProjectResponse>>, AppError> {
    let pagination = PaginationQuery {
        page: query.page,
        per_page: query.per_page,
    };
    let page = pagination.page();
    let per_page = pagination.per_page();
    let projects = project_service::list(
        &tenant_db,
        organization_id,
        query.search.as_deref(),
        page,
        per_page,
    )
    .await?;
    Ok(Json(PaginatedResponse::new(
        projects.data.into_iter().map(project_response).collect(),
        projects.total,
        page,
        per_page,
    )))
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
    Ok(Json(project_response(
        project_service::get(&tenant_db, organization_id, project_id).await?,
    )))
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
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = get_app_state();
    project_service::delete(
        &tenant_db,
        &state.s3_providers,
        &state.secrets,
        organization_id,
        auth.actor_id,
        project_id,
    )
    .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

fn project_response(project: project_service::ProjectRecord) -> ProjectResponse {
    ProjectResponse {
        id: project.id,
        organization_id: project.organization_id,
        name: project.name,
        default_environment_id: project.default_environment_id,
        main_environment: project
            .main_environment
            .map(|environment| EnvironmentResponse {
                id: environment.id,
                name: environment.name,
                is_preview: environment.is_preview,
                draft_timeline: environment.draft_timeline.to_string(),
                deployed_timeline: environment.deployed_timeline.to_string(),
                is_default: environment.is_default,
            }),
        created_at: project.created_at,
        updated_at: project.updated_at,
    }
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/environments",
    params(("organization_id" = Uuid, Path, description = "Organization ID")),
    responses((status = 200, description = "Organization environments", body = Vec<EnvironmentWithProjectResponse>)),
    tag = "environments",
)]
pub async fn list_organization_environments(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
) -> Result<Json<Vec<EnvironmentWithProjectResponse>>, AppError> {
    let environments = project_environments::list_organization(&tenant_db, organization_id).await?;
    Ok(Json(
        environments
            .into_iter()
            .map(|row| EnvironmentWithProjectResponse {
                id: row.environment.id,
                name: row.environment.name,
                is_preview: row.environment.is_preview,
                draft_timeline: row.environment.draft_timeline.to_string(),
                deployed_timeline: row.environment.deployed_timeline.to_string(),
                is_default: row.is_default,
                project_id: row.environment.project_id,
                project_name: row.project_name.unwrap_or_else(|| "Unknown".into()),
            })
            .collect(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/projects/{project_id}/environments",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
    ),
    responses((status = 200, description = "Project environments", body = Vec<EnvironmentResponse>)),
    tag = "environments",
)]
pub async fn list_environments(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<EnvironmentResponse>>, AppError> {
    let environments = project_environments::list(&tenant_db, organization_id, project_id).await?;
    Ok(Json(
        environments
            .into_iter()
            .map(|row| EnvironmentResponse {
                id: row.environment.id,
                name: row.environment.name,
                is_preview: row.environment.is_preview,
                draft_timeline: row.environment.draft_timeline.to_string(),
                deployed_timeline: row.environment.deployed_timeline.to_string(),
                is_default: row.is_default,
            })
            .collect(),
    ))
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
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateEnvironmentRequest>,
) -> Result<(axum::http::StatusCode, Json<EnvironmentResponse>), AppError> {
    let environment = project_environments::create(
        &tenant_db,
        organization_id,
        project_id,
        auth.actor_id,
        &body.name,
        body.parent_timeline_id,
        body.is_preview,
    )
    .await?;
    let environment_row = environment.environment;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(EnvironmentResponse {
            id: environment_row.id,
            name: environment_row.name,
            is_preview: environment_row.is_preview,
            draft_timeline: environment_row.draft_timeline.to_string(),
            deployed_timeline: environment_row.deployed_timeline.to_string(),
            is_default: environment.is_default,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/projects/{project_id}/timelines",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("environment_id" = Option<Uuid>, Query, description = "Origin environment filter"),
        ("limit" = Option<u64>, Query, description = "Page size, default 50, max 100"),
        ("cursor" = Option<String>, Query, description = "Continuation cursor from a previous page"),
        ("anchor_revision_id" = Option<Uuid>, Query, description = "Start at this revision; cannot be combined with cursor; in graph mode selects the page containing it"),
        ("graph" = Option<bool>, Query, description = "Book pagination: fixed revision-order pages instead of a cursor"),
        ("page" = Option<u64>, Query, description = "Graph page number, 0 = newest; ignored when anchor_revision_id is set"),
    ),
    responses((status = 200, description = "Paginated project timelines", body = TimelinePageResponse)),
    tag = "projects",
)]
pub async fn list_project_timelines(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
    axum::extract::Query(query): axum::extract::Query<ListTimelinesQuery>,
) -> Result<Json<TimelinePageResponse>, AppError> {
    let page = project_timelines::list(
        &tenant_db,
        organization_id,
        project_id,
        project_timelines::ListInput {
            environment_id: query.environment_id,
            limit: query.limit,
            cursor: query.cursor,
            anchor_revision_id: query.anchor_revision_id,
            graph: query.graph,
            page: query.page,
        },
    )
    .await?;
    Ok(Json(TimelinePageResponse {
        data: page
            .data
            .into_iter()
            .map(|row| TimelineResponse {
                id: row.id,
                environment_id: row.environment_id,
                timeline: row.timeline,
                name: row.name,
                parent_timeline_id: row.parent_timeline_id,
                created_at: row.created_at,
            })
            .collect(),
        next_cursor: page.next_cursor,
        page: page.page,
        total_pages: page.total_pages,
        has_newer: page.has_newer,
        has_older: page.has_older,
        graph_nodes: page.graph_nodes.map(|nodes| {
            nodes
                .into_iter()
                .map(|node| TimelineGraphNode {
                    id: node.id,
                    timeline: node.timeline,
                    parent_id: node.parent_id,
                    lane: node.lane,
                    child_ids: node.child_ids,
                })
                .collect()
        }),
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateEnvironmentRequest {
    pub name: Option<String>,
    pub draft_timeline_id: Option<Uuid>,
    pub deployed_timeline_id: Option<Uuid>,
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
        (status = 400, description = "Name or revision is required"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Environment name already exists"),
    ),
    tag = "environments",
)]
pub async fn update_environment(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, project_id, environment_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<UpdateEnvironmentRequest>,
) -> Result<Json<EnvironmentResponse>, AppError> {
    let updated = project_environments::update(
        &tenant_db,
        organization_id,
        project_id,
        environment_id,
        auth.actor_id,
        body.name.map(|name| name.trim().to_string()),
        body.draft_timeline_id,
        body.deployed_timeline_id,
    )
    .await?;
    Ok(Json(EnvironmentResponse {
        id: updated.environment.id,
        name: updated.environment.name,
        is_preview: updated.environment.is_preview,
        draft_timeline: updated.environment.draft_timeline.to_string(),
        deployed_timeline: updated.environment.deployed_timeline.to_string(),
        is_default: updated.is_default,
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
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, project_id, environment_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    project_environments::delete(
        &tenant_db,
        organization_id,
        project_id,
        environment_id,
        auth.actor_id,
    )
    .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(Serialize, ToSchema)]
pub struct ResolvedContainerPin {
    pub container_id: Uuid,
    pub container_name: String,
    pub image: String,
    pub external_registry_id: Option<Uuid>,
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

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/projects/{project_id}/timelines/{timeline_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("timeline_id" = Uuid, Path, description = "Timeline ID"),
    ),
    responses(
        (status = 200, description = "Resolved timeline", body = ResolvedTimelineResponse),
        (status = 404, description = "Not found"),
    ),
    tag = "projects",
)]
pub async fn get_timeline(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, project_id, timeline_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<ResolvedTimelineResponse>, AppError> {
    let resolved =
        project_timelines::get(&tenant_db, organization_id, project_id, timeline_id).await?;
    Ok(Json(ResolvedTimelineResponse {
        id: resolved.timeline.id,
        environment_id: resolved.timeline.environment_id,
        timeline: resolved.timeline.timeline,
        name: resolved.timeline.name,
        parent_timeline_id: resolved.timeline.parent_timeline_id,
        containers: resolved
            .containers
            .into_iter()
            .map(|container| ResolvedContainerPin {
                container_id: container.container_id,
                container_name: container.container_name,
                image: container.image,
                external_registry_id: container.external_registry_id,
            })
            .collect(),
        created_at: resolved.timeline.created_at,
    }))
}
