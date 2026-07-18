use axum::{Json, extract::Path};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

use super::databases::verify_org_access;
use super::projects::default_true;
use crate::errors::AppError;
use crate::middleware::auth::AuthContext;
use crate::models::entities::{container, container_version, project_branch, project_timeline};
use crate::models::pins::TimelinePins;
use crate::services::{events, revisions};

#[derive(Deserialize, ToSchema)]
pub struct CreateContainerRequest {
    pub name: String,
    pub image: String,
    pub project_id: Uuid,
    pub branch_id: Uuid,
    #[serde(default)]
    pub public: bool,
    #[serde(default = "default_replica_count")]
    pub replica_count: i32,
    pub port: Option<i32>,
    pub env: Option<serde_json::Value>,
    pub resources: Option<serde_json::Value>,
    pub pull_secret_id: Option<Uuid>,
    pub health_check: Option<serde_json::Value>,
    pub region_id: Uuid,
}

fn default_replica_count() -> i32 {
    1
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateContainerRequest {
    pub name: Option<String>,
    pub image: Option<String>,
    pub public: Option<bool>,
    pub replica_count: Option<i32>,
    pub port: Option<i32>,
    pub env: Option<serde_json::Value>,
    pub resources: Option<serde_json::Value>,
    pub pull_secret_id: Option<Uuid>,
    pub health_check: Option<serde_json::Value>,
    #[serde(default = "default_true")]
    pub auto_deploy: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct ContainerActionQuery {
    pub branch_id: Uuid,
}

#[derive(Deserialize, ToSchema)]
pub struct ListContainersQuery {
    pub project_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
pub struct ContainerResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub current_version: Option<ContainerVersionResponse>,
    pub project_id: Option<Uuid>,
    pub region_id: Uuid,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct ContainerVersionResponse {
    pub id: Uuid,
    pub version: i32,
    pub image: String,
    pub public: bool,
    pub replica_count: i32,
    pub port: Option<i32>,
    pub env: Option<serde_json::Value>,
    pub resources: Option<serde_json::Value>,
    pub pull_secret_id: Option<Uuid>,
    pub health_check: Option<serde_json::Value>,
    pub created_at: String,
}

fn resolve_latest_version(version: &container_version::Model) -> ContainerVersionResponse {
    ContainerVersionResponse {
        id: version.id,
        version: version.version,
        image: version.image.clone(),
        public: version.public,
        replica_count: version.replica_count,
        port: version.port,
        env: version.env.clone(),
        resources: version.resources.clone(),
        pull_secret_id: version.pull_secret_id,
        health_check: version.health_check.clone(),
        created_at: version.created_at.to_string(),
    }
}

fn has_config_change(req: &UpdateContainerRequest) -> bool {
    req.image.is_some()
        || req.public.is_some()
        || req.replica_count.is_some()
        || req.port.is_some()
        || req.env.is_some()
        || req.resources.is_some()
        || req.pull_secret_id.is_some()
        || req.health_check.is_some()
}

async fn get_branch(
    tx: &impl sea_orm::ConnectionTrait,
    branch_id: Uuid,
    organization_id: Uuid,
    project_id: Option<Uuid>,
) -> Result<project_branch::Model, AppError> {
    let mut query = project_branch::Entity::find()
        .filter(project_branch::Column::Id.eq(branch_id))
        .filter(project_branch::Column::OrganizationId.eq(organization_id));
    if let Some(project_id) = project_id {
        query = query.filter(project_branch::Column::ProjectId.eq(project_id));
    }
    query
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Branch not found".into()))
}

async fn get_branch_timeline_pins(
    tx: &impl sea_orm::ConnectionTrait,
    branch: &project_branch::Model,
) -> Result<TimelinePins, AppError> {
    let head = project_timeline::Entity::find_by_id(branch.timeline)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Branch timeline not found".into()))?;
    Ok(TimelinePins::from_json_value(&head.pins))
}

fn build_response(
    container: &container::Model,
    version: &container_version::Model,
) -> ContainerResponse {
    ContainerResponse {
        id: container.id,
        organization_id: container.organization_id,
        name: container.name.clone(),
        current_version: Some(resolve_latest_version(version)),
        project_id: Some(container.project_id),
        region_id: container.region_id,
        created_at: container.created_at.to_string(),
        updated_at: container.updated_at.to_string(),
    }
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/containers",
    request_body = CreateContainerRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
    ),
    responses(
        (status = 201, description = "Container created", body = ContainerResponse),
        (status = 404, description = "Not found"),
    ),
    tag = "containers",
)]
pub async fn create_container(
    AuthContext { tenant_db, auth }: AuthContext,
    Path(organization_id): Path<Uuid>,
    Json(body): Json<CreateContainerRequest>,
) -> Result<(axum::http::StatusCode, Json<ContainerResponse>), AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name is required".into()));
    }
    let image = body.image.trim().to_string();
    if image.is_empty() {
        return Err(AppError::BadRequest("Image is required".into()));
    }
    if body.region_id.is_nil() {
        return Err(AppError::BadRequest("Region is required".into()));
    }

    let container_id = Uuid::new_v4();
    let version_id = Uuid::new_v4();

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let branch = get_branch(tx, body.branch_id, organization_id, Some(body.project_id)).await?;

    let created_container: container::Model = container::ActiveModel {
        id: Set(container_id),
        project_id: Set(body.project_id),
        organization_id: Set(organization_id),
        name: Set(name.clone()),
        region_id: Set(body.region_id),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;

    let created_version: container_version::Model = container_version::ActiveModel {
        id: Set(version_id),
        container_id: Set(container_id),
        organization_id: Set(organization_id),
        version: Set(1),
        image: Set(image.clone()),
        public: Set(body.public),
        replica_count: Set(body.replica_count),
        port: Set(body.port),
        env: Set(body.env.clone()),
        resources: Set(body.resources.clone()),
        pull_secret_id: Set(body.pull_secret_id),
        health_check: Set(body.health_check.clone()),
        created_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;

    let mut pins = get_branch_timeline_pins(tx, &branch).await?;
    pins.set_container(container_id, version_id);
    revisions::create_revision(
        tx,
        &branch,
        &pins,
        Some(format!("Created container '{}'", name)),
        true,
    )
    .await?;
    events::record(tx, organization_id, body.project_id, "container:created", serde_json::json!({"summary": format!("Created container '{}'", name), "target_id": container_id.to_string(), "branch_id": branch.id.to_string()}), auth.actor_id).await?;

    scoped.commit().await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(build_response(&created_container, &created_version)),
    ))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/containers",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Option<Uuid>, Query, description = "Filter by project"),
        ("branch_id" = Option<Uuid>, Query, description = "Filter by branch"),
    ),
    responses(
        (status = 200, description = "List of containers", body = Vec<ContainerResponse>),
    ),
    tag = "containers",
)]
pub async fn list_containers(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
    axum::extract::Query(query): axum::extract::Query<ListContainersQuery>,
) -> Result<Json<Vec<ContainerResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let branch = if let Some(branch_id) = query.branch_id {
        Some(get_branch(tx, branch_id, organization_id, query.project_id).await?)
    } else if let Some(project_id) = query.project_id {
        Some(find_main_branch_containers(tx, project_id, organization_id).await?)
    } else {
        None
    };

    if let Some(branch) = branch {
        let pins = get_branch_timeline_pins(tx, &branch).await?;

        if pins.container.is_empty() {
            scoped.commit().await?;
            return Ok(Json(Vec::new()));
        }

        let container_ids: Vec<Uuid> = pins.container.keys().cloned().collect();
        let pinned_version_ids: Vec<Uuid> = pins.container.values().cloned().collect();

        let containers = container::Entity::find()
            .filter(container::Column::Id.is_in(container_ids))
            .all(tx)
            .await?;

        let versions = container_version::Entity::find()
            .filter(container_version::Column::Id.is_in(pinned_version_ids))
            .all(tx)
            .await?;

        let version_map: HashMap<Uuid, &container_version::Model> =
            versions.iter().map(|v| (v.id, v)).collect();

        let mut responses = Vec::new();
        for c in containers {
            if let Some(v) = pins
                .container
                .get(&c.id)
                .and_then(|vid| version_map.get(vid))
            {
                responses.push(build_response(&c, v));
            }
        }

        scoped.commit().await?;
        return Ok(Json(responses));
    }

    let containers = container::Entity::find()
        .order_by_asc(container::Column::Name)
        .all(tx)
        .await?;

    if containers.is_empty() {
        scoped.commit().await?;
        return Ok(Json(Vec::new()));
    }

    let container_ids: Vec<Uuid> = containers.iter().map(|c| c.id).collect();

    let all_versions = container_version::Entity::find()
        .filter(container_version::Column::ContainerId.is_in(container_ids))
        .order_by_desc(container_version::Column::Version)
        .all(tx)
        .await?;

    let latest: HashMap<Uuid, &container_version::Model> =
        all_versions.iter().fold(HashMap::new(), |mut acc, v| {
            acc.entry(v.container_id).or_insert(v);
            acc
        });

    scoped.commit().await?;

    let responses = containers
        .iter()
        .filter_map(|c| latest.get(&c.id).map(|v| build_response(c, v)))
        .collect();

    Ok(Json(responses))
}

async fn find_main_branch_containers(
    tx: &impl sea_orm::ConnectionTrait,
    project_id: Uuid,
    organization_id: Uuid,
) -> Result<project_branch::Model, AppError> {
    project_branch::Entity::find()
        .filter(project_branch::Column::ProjectId.eq(project_id))
        .filter(project_branch::Column::OrganizationId.eq(organization_id))
        .filter(project_branch::Column::Name.eq("main"))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Main branch not found for project".into()))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/containers/{container_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("container_id" = Uuid, Path, description = "Container ID"),
    ),
    responses(
        (status = 200, description = "Container details", body = ContainerResponse),
        (status = 404, description = "Not found"),
    ),
    tag = "containers",
)]
pub async fn get_container(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, container_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ContainerResponse>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let c = container::Entity::find()
        .filter(container::Column::Id.eq(container_id))
        .filter(container::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Container not found".into()))?;

    let latest = container_version::Entity::find()
        .filter(container_version::Column::ContainerId.eq(c.id))
        .order_by_desc(container_version::Column::Version)
        .one(tx)
        .await?;

    scoped.commit().await?;

    Ok(Json(ContainerResponse {
        id: c.id,
        organization_id: c.organization_id,
        name: c.name,
        current_version: latest.map(|v| resolve_latest_version(&v)),
        project_id: Some(c.project_id),
        region_id: c.region_id,
        created_at: c.created_at.to_string(),
        updated_at: c.updated_at.to_string(),
    }))
}

#[utoipa::path(
    patch,
    path = "/api/organization/{organization_id}/containers/{container_id}",
    request_body = UpdateContainerRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("container_id" = Uuid, Path, description = "Container ID"),
        ("branch_id" = Uuid, Query, description = "Branch ID for the revision"),
    ),
    responses(
        (status = 200, description = "Container updated", body = ContainerResponse),
        (status = 404, description = "Not found"),
    ),
    tag = "containers",
)]
pub async fn update_container(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, container_id)): Path<(Uuid, Uuid)>,
    axum::extract::Query(action): axum::extract::Query<ContainerActionQuery>,
    Json(body): Json<UpdateContainerRequest>,
) -> Result<Json<ContainerResponse>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let c = container::Entity::find()
        .filter(container::Column::Id.eq(container_id))
        .filter(container::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Container not found".into()))?;

    let branch = get_branch(tx, action.branch_id, organization_id, Some(c.project_id)).await?;

    if let Some(ref new_name) = body.name {
        let trimmed = new_name.trim().to_string();
        if !trimmed.is_empty() {
            let mut active: container::ActiveModel = c.clone().into();
            active.name = Set(trimmed);
            active.updated_at = Set(Utc::now().fixed_offset());
            active.update(tx).await?;
        }
    }

    let mut new_version: Option<container_version::Model> = None;

    if has_config_change(&body) {
        let latest = container_version::Entity::find()
            .filter(container_version::Column::ContainerId.eq(container_id))
            .order_by_desc(container_version::Column::Version)
            .one(tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Container version not found".into()))?;

        let next_ver = latest.version + 1;
        let version_id = Uuid::new_v4();

        let cv = container_version::ActiveModel {
            id: Set(version_id),
            container_id: Set(container_id),
            organization_id: Set(organization_id),
            version: Set(next_ver),
            image: Set(body.image.unwrap_or_else(|| latest.image.clone())),
            public: Set(body.public.unwrap_or(latest.public)),
            replica_count: Set(body.replica_count.unwrap_or(latest.replica_count)),
            port: Set(body.port.or(latest.port)),
            env: Set(body.env.clone().or(latest.env.clone())),
            resources: Set(body.resources.clone().or(latest.resources.clone())),
            pull_secret_id: Set(body.pull_secret_id.or(latest.pull_secret_id)),
            health_check: Set(body.health_check.clone().or(latest.health_check.clone())),
            created_at: Set(Utc::now().fixed_offset()),
        };

        new_version = Some(cv.insert(tx).await?);

        let mut pins = get_branch_timeline_pins(tx, &branch).await?;
        pins.set_container(container_id, version_id);
        revisions::create_revision(
            tx,
            &branch,
            &pins,
            Some("Updated container configuration".into()),
            body.auto_deploy,
        )
        .await?;
    }

    events::record(tx, organization_id, c.project_id, "container:updated", serde_json::json!({"summary": format!("Updated container '{}'", c.name), "target_id": container_id.to_string(), "branch_id": branch.id.to_string()}), auth.actor_id).await?;

    scoped.commit().await?;

    let scoped2 = tenant_db.begin_scoped_transaction().await?;
    let tx2 = scoped2.connection();

    let updated = container::Entity::find_by_id(container_id)
        .one(tx2)
        .await?
        .unwrap();

    let latest_version = if let Some(v) = new_version {
        Some(v)
    } else {
        container_version::Entity::find()
            .filter(container_version::Column::ContainerId.eq(container_id))
            .order_by_desc(container_version::Column::Version)
            .one(tx2)
            .await?
    };

    scoped2.commit().await?;

    Ok(Json(ContainerResponse {
        id: updated.id,
        organization_id: updated.organization_id,
        name: updated.name,
        current_version: latest_version.map(|v| resolve_latest_version(&v)),
        project_id: Some(c.project_id),
        region_id: updated.region_id,
        created_at: updated.created_at.to_string(),
        updated_at: updated.updated_at.to_string(),
    }))
}

#[utoipa::path(
    delete,
    path = "/api/organization/{organization_id}/containers/{container_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("container_id" = Uuid, Path, description = "Container ID"),
        ("branch_id" = Uuid, Query, description = "Branch ID for the revision"),
    ),
    responses(
        (status = 200, description = "Container deleted"),
        (status = 404, description = "Not found"),
    ),
    tag = "containers",
)]
pub async fn delete_container(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, container_id)): Path<(Uuid, Uuid)>,
    axum::extract::Query(action): axum::extract::Query<ContainerActionQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let c = container::Entity::find()
        .filter(container::Column::Id.eq(container_id))
        .filter(container::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Container not found".into()))?;

    let branch = get_branch(tx, action.branch_id, organization_id, Some(c.project_id)).await?;

    let mut pins = get_branch_timeline_pins(tx, &branch).await?;
    pins.remove_container(&container_id);
    revisions::create_revision(
        tx,
        &branch,
        &pins,
        Some(format!("Removed container '{}'", c.name)),
        true,
    )
    .await?;
    events::record(tx, organization_id, c.project_id, "container:removed", serde_json::json!({"summary": format!("Removed container '{}'", c.name), "target_id": container_id.to_string(), "branch_id": branch.id.to_string()}), auth.actor_id).await?;

    scoped.commit().await?;

    Ok(Json(serde_json::json!({ "success": true })))
}
