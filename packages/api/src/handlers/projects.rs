use axum::{Json, extract::Path};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use utoipa::ToSchema;
use uuid::Uuid;

use super::databases::{verify_org_access, verify_org_owner};
use crate::errors::AppError;
use crate::middleware::auth::AuthContext;
use crate::models::entities::{
    credential, project, project_environment, project_timeline, secret, storage,
    storage_access_token,
};
use crate::models::manifest::RevisionManifest;
use crate::services::agent;
use crate::services::buckets;
use crate::services::events;
use crate::services::revisions;
use crate::state::get_app_state;
use crate::utils::pagination::{PaginatedResponse, PaginationQuery};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

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

#[derive(Serialize, Deserialize)]
struct TimelineCursor {
    project_id: Uuid,
    environment_id: Option<Uuid>,
    timeline: i32,
}

fn encode_timeline_cursor(project_id: Uuid, environment_id: Option<Uuid>, timeline: i32) -> String {
    let cursor = TimelineCursor {
        project_id,
        environment_id,
        timeline,
    };
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&cursor).unwrap_or_default())
}

fn decode_timeline_cursor(
    cursor: &str,
    project_id: Uuid,
    environment_id: Option<Uuid>,
) -> Result<TimelineCursor, AppError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| AppError::BadRequest("Invalid timeline cursor".into()))?;
    let decoded: TimelineCursor = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::BadRequest("Invalid timeline cursor".into()))?;
    if decoded.project_id != project_id || decoded.environment_id != environment_id {
        return Err(AppError::BadRequest(
            "Timeline cursor does not match this request".into(),
        ));
    }
    Ok(decoded)
}

impl From<project_timeline::Model> for TimelineResponse {
    fn from(t: project_timeline::Model) -> Self {
        TimelineResponse {
            id: t.id,
            environment_id: t.environment_id,
            timeline: t.timeline,
            name: t.name,
            parent_timeline_id: t.parent_timeline_id,
            created_at: t.created_at.to_string(),
        }
    }
}

fn clamp_book_page(page: u64, total: u64, limit: usize) -> u64 {
    let total_pages = total.div_ceil(limit as u64).max(1);
    page.min(total_pages - 1)
}

fn layout_timeline_book(
    rows: Vec<(Uuid, i32, Option<Uuid>)>,
    main_head: Option<Uuid>,
) -> Vec<TimelineGraphNode> {
    let mut nodes: Vec<_> = rows
        .into_iter()
        .map(|(id, timeline, parent_id)| TimelineGraphNode {
            id,
            timeline,
            parent_id,
            lane: 0,
            child_ids: Vec::new(),
        })
        .collect();
    let positions: HashMap<_, _> = nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
    let mut main = HashSet::new();
    let mut current = main_head;
    while let Some(index) = current.and_then(|id| positions.get(&id).copied()) {
        if !main.insert(nodes[index].id) {
            break;
        }
        current = nodes[index].parent_id;
    }
    let mut continued = HashSet::new();
    for node in &nodes {
        if main.contains(&node.id) {
            if let Some(parent) = node.parent_id {
                continued.insert(parent);
            }
        }
    }
    let mut next_lane = usize::from(!main.is_empty());
    for i in (0..nodes.len()).rev() {
        let parent = nodes[i]
            .parent_id
            .and_then(|id| positions.get(&id).copied());
        if !main.contains(&nodes[i].id) {
            nodes[i].lane = match parent {
                Some(p) if continued.insert(nodes[p].id) => nodes[p].lane,
                _ => {
                    let lane = next_lane;
                    next_lane += 1;
                    lane
                }
            };
        }
        if let Some(p) = parent {
            let id = nodes[i].id;
            nodes[p].child_ids.push(id);
        }
    }
    nodes
}

async fn list_project_timeline_book(
    tx: &impl ConnectionTrait,
    organization_id: Uuid,
    project_id: Uuid,
    project: &project::Model,
    limit: usize,
    requested_page: u64,
    anchor_revision_id: Option<Uuid>,
) -> Result<TimelinePageResponse, AppError> {
    use project_timeline::{Column, Entity};

    let rows = Entity::find()
        .select_only()
        .columns([Column::Id, Column::Timeline, Column::ParentTimelineId])
        .filter(Column::ProjectId.eq(project_id))
        .filter(Column::OrganizationId.eq(organization_id))
        .order_by_desc(Column::Timeline)
        .into_tuple::<(Uuid, i32, Option<Uuid>)>()
        .all(tx)
        .await?;
    let main_head = if let Some(id) = project.default_environment_id {
        project_environment::Entity::find_by_id(id)
            .one(tx)
            .await?
            .map(|env| env.draft_timeline)
    } else {
        None
    };
    let book = layout_timeline_book(rows, main_head);
    let page = match anchor_revision_id {
        Some(id) => {
            book.iter().position(|node| node.id == id).ok_or_else(|| {
                AppError::NotFound("Anchor revision not found in this book".into())
            })? as u64
                / limit as u64
        }
        None => clamp_book_page(requested_page, book.len() as u64, limit),
    };
    let offset = page as usize * limit;
    let page_nodes: Vec<_> = book.iter().skip(offset).take(limit).cloned().collect();
    let ids: Vec<_> = page_nodes.iter().map(|node| node.id).collect();
    let revisions = if ids.is_empty() {
        Vec::new()
    } else {
        Entity::find()
            .filter(Column::ProjectId.eq(project_id))
            .filter(Column::OrganizationId.eq(organization_id))
            .filter(Column::Id.is_in(ids))
            .order_by_desc(Column::Timeline)
            .all(tx)
            .await?
    };
    Ok(TimelinePageResponse {
        data: revisions.into_iter().map(TimelineResponse::from).collect(),
        next_cursor: None,
        page: Some(page),
        total_pages: Some((book.len() as u64).div_ceil(limit as u64).max(1)),
        has_newer: Some(page > 0),
        has_older: Some(offset + limit < book.len()),
        graph_nodes: Some(book),
    })
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
    verify_org_access(&tenant_db, organization_id)?;

    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name is required".into()));
    }

    let project_id = Uuid::new_v4();
    let environment_id = Uuid::new_v4();

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

    let main_environment: project_environment::Model = project_environment::ActiveModel {
        id: Set(environment_id),
        project_id: Set(project_id),
        organization_id: Set(organization_id),
        name: Set("main".into()),
        is_preview: Set(false),
        draft_timeline: Set(timeline.id),
        deployed_timeline: Set(timeline.id),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;

    let mut project_active: project::ActiveModel = created.clone().into();
    project_active.default_environment_id = Set(Some(environment_id));
    project_active.updated_at = Set(Utc::now().fixed_offset());
    let updated_project: project::Model = project_active.update(tx).await?;

    events::record(
        tx,
        organization_id,
        project_id,
        "project:created",
        serde_json::json!({
            "summary": format!("Created project '{}'", updated_project.name),
            "target_id": project_id.to_string(),
        }),
        auth.actor_id,
    )
    .await?;

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
                is_preview: main_environment.is_preview,
                draft_timeline: main_environment.draft_timeline.to_string(),
                deployed_timeline: main_environment.deployed_timeline.to_string(),
                is_default: true,
            }),
            created_at: updated_project.created_at.to_string(),
            updated_at: updated_project.updated_at.to_string(),
        }),
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
                is_preview: b.is_preview,
                draft_timeline: b.draft_timeline.to_string(),
                deployed_timeline: b.deployed_timeline.to_string(),
                is_default: p.default_environment_id == Some(b.id),
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
                is_preview: b.is_preview,
                draft_timeline: b.draft_timeline.to_string(),
                deployed_timeline: b.deployed_timeline.to_string(),
                is_default: p.default_environment_id == Some(b.id),
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
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, project_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    verify_org_owner(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    use project::{Column, Entity};
    let project = Entity::find()
        .filter(Column::Id.eq(project_id))
        .filter(Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".into()))?;
    let tokens = storage_access_token::Entity::find()
        .filter(storage_access_token::Column::ProjectId.eq(project_id))
        .find_also_related(credential::Entity)
        .all(tx)
        .await?;
    let access_keys = tokens
        .iter()
        .filter_map(|(_, credential)| credential.as_ref().map(|c| c.access_key_id.clone()))
        .collect::<Vec<_>>();
    let secret_ids = tokens
        .iter()
        .filter_map(|(_, credential)| credential.as_ref().map(|c| c.secret_id))
        .collect::<Vec<_>>();
    storage_access_token::Entity::delete_many()
        .filter(storage_access_token::Column::ProjectId.eq(project_id))
        .exec(tx)
        .await?;
    secret::Entity::delete_many()
        .filter(secret::Column::Id.is_in(secret_ids))
        .exec(tx)
        .await?;
    let storage_buckets = storage::Entity::find()
        .filter(storage::Column::ProjectId.eq(project_id))
        .all(tx)
        .await?;
    for bucket in storage_buckets {
        buckets::delete(tx, bucket.bucket_id).await?;
    }

    Entity::delete_by_id(project_id).exec(tx).await?;
    events::record(
        tx,
        organization_id,
        project_id,
        "project:deleted",
        serde_json::json!({
            "summary": format!("Deleted project '{}'", project.name),
            "target_id": project_id.to_string(),
        }),
        auth.actor_id,
    )
    .await?;
    scoped.commit().await?;
    if let Err(error) = get_app_state()
        .s3_providers
        .invalidate_access_token_caches(&access_keys)
        .await
    {
        tracing::warn!(%error, %project_id, "project cache invalidation failed after deletion");
    }

    Ok(Json(serde_json::json!({ "success": true })))
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
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let environments = project_environment::Entity::find()
        .filter(project_environment::Column::OrganizationId.eq(organization_id))
        .order_by_asc(project_environment::Column::Name)
        .find_also_related(project::Entity)
        .all(tx)
        .await?;

    scoped.commit().await?;

    let responses = environments
        .into_iter()
        .map(|(environment, project)| {
            let is_default = project
                .as_ref()
                .is_some_and(|project| project.default_environment_id == Some(environment.id));
            EnvironmentWithProjectResponse {
                id: environment.id,
                name: environment.name,
                is_preview: environment.is_preview,
                draft_timeline: environment.draft_timeline.to_string(),
                deployed_timeline: environment.deployed_timeline.to_string(),
                is_default,
                project_id: environment.project_id,
                project_name: project
                    .map(|project| project.name)
                    .unwrap_or_else(|| "Unknown".into()),
            }
        })
        .collect();

    Ok(Json(responses))
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

    scoped.commit().await?;

    let responses = environments
        .into_iter()
        .map(|b| EnvironmentResponse {
            id: b.id,
            name: b.name,
            is_preview: b.is_preview,
            draft_timeline: b.draft_timeline.to_string(),
            deployed_timeline: b.deployed_timeline.to_string(),
            is_default: project.default_environment_id == Some(b.id),
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
    AuthContext { tenant_db, auth }: AuthContext,
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

    let environment_id = Uuid::new_v4();

    let timeline_id = if let Some(parent_timeline_id) = body.parent_timeline_id {
        let parent = project_timeline::Entity::find()
            .filter(project_timeline::Column::Id.eq(parent_timeline_id))
            .filter(project_timeline::Column::ProjectId.eq(project_id))
            .one(tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Timeline revision not found".into()))?;
        revisions::load_manifest(tx, parent.manifest_id).await?;
        parent.id
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

    let environment: project_environment::Model = project_environment::ActiveModel {
        id: Set(environment_id),
        project_id: Set(project_id),
        organization_id: Set(organization_id),
        name: Set(name),
        is_preview: Set(body.is_preview),
        draft_timeline: Set(timeline_id),
        deployed_timeline: Set(timeline_id),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;

    events::record(
        tx,
        organization_id,
        project_id,
        "environment:created",
        serde_json::json!({
            "summary": format!("Created environment '{}'", environment.name),
            "target_id": environment.id.to_string(),
        }),
        auth.actor_id,
    )
    .await?;

    scoped.commit().await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(EnvironmentResponse {
            id: environment.id,
            name: environment.name,
            is_preview: environment.is_preview,
            draft_timeline: environment.draft_timeline.to_string(),
            deployed_timeline: environment.deployed_timeline.to_string(),
            is_default: project.default_environment_id == Some(environment.id),
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
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let project = project::Entity::find()
        .filter(project::Column::Id.eq(project_id))
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".into()))?;

    let limit = query.limit.unwrap_or(50).clamp(1, 100) as usize;

    if query.graph.unwrap_or(false) {
        if query.cursor.is_some() || query.environment_id.is_some() {
            return Err(AppError::BadRequest(
                "Graph pages do not support cursors or environment filters".into(),
            ));
        }
        let page = list_project_timeline_book(
            tx,
            organization_id,
            project_id,
            &project,
            limit,
            query.page.unwrap_or(0),
            query.anchor_revision_id,
        )
        .await?;
        scoped.commit().await?;
        return Ok(Json(page));
    }

    if query.cursor.is_some() && query.anchor_revision_id.is_some() {
        return Err(AppError::BadRequest(
            "Use either a cursor or an anchor revision".into(),
        ));
    }
    let cursor = query
        .cursor
        .as_deref()
        .map(|cursor| decode_timeline_cursor(cursor, project_id, query.environment_id))
        .transpose()?;

    use project_timeline::{Column, Entity};
    let mut select = Entity::find().filter(Column::ProjectId.eq(project_id));
    if let Some(anchor) = query.anchor_revision_id {
        let anchor = Entity::find_by_id(anchor)
            .filter(Column::ProjectId.eq(project_id))
            .one(tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Anchor revision not found".into()))?;
        select = select.filter(Column::Timeline.lte(anchor.timeline));
    }

    if let Some(environment_id) = query.environment_id {
        select = select.filter(Column::EnvironmentId.eq(environment_id));
    }
    if let Some(cursor) = cursor {
        select = select.filter(Column::Timeline.lt(cursor.timeline));
    }

    let mut timelines = select
        .order_by_desc(Column::Timeline)
        .limit(limit as u64 + 1)
        .all(tx)
        .await?;

    let has_more = timelines.len() > limit;
    timelines.truncate(limit);
    let next_cursor = if has_more {
        timelines
            .last()
            .map(|t| encode_timeline_cursor(project_id, query.environment_id, t.timeline))
    } else {
        None
    };

    scoped.commit().await?;

    let data = timelines.into_iter().map(TimelineResponse::from).collect();

    Ok(Json(TimelinePageResponse {
        data,
        next_cursor,
        ..Default::default()
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
    verify_org_access(&tenant_db, organization_id)?;

    let name = body.name.map(|name| name.trim().to_string());
    if name.as_ref().is_some_and(String::is_empty) {
        return Err(AppError::BadRequest("Name is required".into()));
    }
    if name.is_none() && body.draft_timeline_id.is_none() && body.deployed_timeline_id.is_none() {
        return Err(AppError::BadRequest("Name or revision is required".into()));
    }

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let project = project::Entity::find()
        .filter(project::Column::Id.eq(project_id))
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".into()))?;

    revisions::lock_project(tx, project_id).await?;
    for timeline_id in [body.draft_timeline_id, body.deployed_timeline_id]
        .into_iter()
        .flatten()
    {
        let timeline = project_timeline::Entity::find()
            .filter(project_timeline::Column::Id.eq(timeline_id))
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
    if let Some(timeline_id) = body.draft_timeline_id {
        active.draft_timeline = Set(timeline_id);
    }
    if let Some(timeline_id) = body.deployed_timeline_id {
        active.deployed_timeline = Set(timeline_id);
    }
    active.updated_at = Set(Utc::now().fixed_offset());
    let updated = active.update(tx).await?;

    events::record(
        tx,
        organization_id,
        project_id,
        "environment:updated",
        serde_json::json!({
            "summary": format!("Updated environment '{}'", updated.name),
            "target_id": updated.id.to_string(),
        }),
        auth.actor_id,
    )
    .await?;

    scoped.commit().await?;
    if let Some(timeline_id) = body.deployed_timeline_id {
        agent::emit_compute(project.id, organization_id, environment_id, timeline_id).await?;
    }

    Ok(Json(EnvironmentResponse {
        id: updated.id,
        name: updated.name,
        is_preview: updated.is_preview,
        draft_timeline: updated.draft_timeline.to_string(),
        deployed_timeline: updated.deployed_timeline.to_string(),
        is_default: project.default_environment_id == Some(updated.id),
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
    verify_org_access(&tenant_db, organization_id)?;
    verify_org_owner(&tenant_db, organization_id)?;

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

    events::record(
        tx,
        organization_id,
        project_id,
        "environment:deleted",
        serde_json::json!({
            "summary": format!("Deleted environment '{}'", environment.name),
            "target_id": environment.id.to_string(),
        }),
        auth.actor_id,
    )
    .await?;

    scoped.commit().await?;

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
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let t = project_timeline::Entity::find()
        .filter(project_timeline::Column::Id.eq(timeline_id))
        .filter(project_timeline::Column::ProjectId.eq(project_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Timeline not found".into()))?;

    let manifest = revisions::load_manifest(tx, t.manifest_id).await?;

    let mut containers: Vec<_> = manifest
        .containers
        .iter()
        .map(|(id, config)| ResolvedContainerPin {
            container_id: *id,
            container_name: config.name.clone(),
            image: config.resolved_image.clone(),
            external_registry_id: config.external_registry_id,
        })
        .collect();
    containers.sort_by(|a, b| {
        a.container_name
            .cmp(&b.container_name)
            .then(a.container_id.cmp(&b.container_id))
    });

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
