use axum::{Json, extract::Path};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

use super::{databases::verify_org_access, external_registries::find_registry};
use crate::errors::AppError;
use crate::middleware::auth::{AuthContext, RequestAuthContext};
use crate::models::entities::{
    container, container_version, project_environment, project_timeline,
};
use crate::models::pins::TimelinePins;
use crate::services::{agent, events, images, revisions};
use crate::state::TenantDatabase;

#[derive(Deserialize, ToSchema)]
pub struct CreateContainerRequest {
    pub name: String,
    pub image: String,
    pub project_id: Uuid,
    pub environment_id: Uuid,
    #[serde(default)]
    pub public: bool,
    #[serde(default = "default_replica_count")]
    pub replica_count: i32,
    pub port: Option<i32>,
    pub env: Option<serde_json::Value>,
    pub resources: Option<serde_json::Value>,
    pub external_registry_id: Option<Uuid>,
    pub health_check: Option<serde_json::Value>,
    #[serde(default)]
    pub auto_deploy: bool,
    pub region_id: Uuid,
}

fn default_replica_count() -> i32 {
    1
}

fn deserialize_present_nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Default, Deserialize, ToSchema)]
pub struct UpdateContainerRequest {
    pub name: Option<String>,
    pub image: Option<String>,
    pub public: Option<bool>,
    pub replica_count: Option<i32>,
    pub port: Option<i32>,
    pub env: Option<serde_json::Value>,
    pub resources: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "deserialize_present_nullable")]
    pub external_registry_id: Option<Option<Uuid>>,
    pub health_check: Option<serde_json::Value>,
    #[serde(default)]
    pub auto_deploy: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct ContainerActionQuery {
    pub environment_id: Uuid,
    pub timeline_id: Uuid,
    #[serde(default)]
    pub deploy: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct ListContainersQuery {
    pub project_id: Option<Uuid>,
    pub environment_id: Option<Uuid>,
    pub timeline_id: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
pub struct ContainerDetailQuery {
    pub environment_id: Option<Uuid>,
    pub timeline_id: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateContainerQuery {
    pub environment_id: Uuid,
    pub timeline_id: Uuid,
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
    pub resolved_image: String,
    pub public: bool,
    pub replica_count: i32,
    pub port: Option<i32>,
    pub env: Option<serde_json::Value>,
    pub resources: Option<serde_json::Value>,
    pub external_registry_id: Option<Uuid>,
    pub health_check: Option<serde_json::Value>,
    pub created_at: String,
}

fn resolve_latest_version(version: &container_version::Model) -> ContainerVersionResponse {
    ContainerVersionResponse {
        id: version.id,
        version: version.version,
        image: version.image.clone(),
        resolved_image: version.resolved_image.clone(),
        public: version.public,
        replica_count: version.replica_count,
        port: version.port,
        env: version.env.clone(),
        resources: version.resources.clone(),
        external_registry_id: version.external_registry_id,
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
        || req.external_registry_id.is_some()
        || req.health_check.is_some()
}

fn is_exact_latest_image(image: &str) -> bool {
    image.ends_with(":latest")
}

pub(crate) fn validate_replica_count(replica_count: i32) -> Result<(), AppError> {
    if !(1..=100).contains(&replica_count) {
        return Err(AppError::BadRequest(
            "Replica count must be between 1 and 100".into(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_port(port: i32) -> Result<(), AppError> {
    if !(1..=65535).contains(&port) {
        return Err(AppError::BadRequest(
            "Port must be between 1 and 65535".into(),
        ));
    }
    Ok(())
}

async fn selected_external_registry(
    tx: &impl sea_orm::ConnectionTrait,
    organization_id: Uuid,
    registry_id: Option<Uuid>,
) -> Result<Option<crate::models::entities::external_registry::Model>, AppError> {
    let Some(registry_id) = registry_id else {
        return Ok(None);
    };
    find_registry(tx, organization_id, registry_id)
        .await
        .map(Some)
}

async fn get_environment(
    tx: &impl sea_orm::ConnectionTrait,
    environment_id: Uuid,
    organization_id: Uuid,
    project_id: Option<Uuid>,
) -> Result<project_environment::Model, AppError> {
    let mut query = project_environment::Entity::find()
        .filter(project_environment::Column::Id.eq(environment_id))
        .filter(project_environment::Column::OrganizationId.eq(organization_id));
    if let Some(project_id) = project_id {
        query = query.filter(project_environment::Column::ProjectId.eq(project_id));
    }
    query
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Environment not found".into()))
}

async fn get_environment_timeline_pins(
    tx: &impl sea_orm::ConnectionTrait,
    environment: &project_environment::Model,
) -> Result<TimelinePins, AppError> {
    let head = project_timeline::Entity::find_by_id(environment.draft_timeline)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Environment timeline not found".into()))?;
    Ok(TimelinePins::from_json_value(&head.pins))
}

async fn get_project_revision_pins(
    tx: &impl sea_orm::ConnectionTrait,
    project_id: Uuid,
    timeline_id: Uuid,
) -> Result<TimelinePins, AppError> {
    let timeline = project_timeline::Entity::find()
        .filter(project_timeline::Column::Id.eq(timeline_id))
        .filter(project_timeline::Column::ProjectId.eq(project_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Timeline revision is not in this project".into()))?;

    Ok(TimelinePins::from_json_value(&timeline.pins))
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
    validate_replica_count(body.replica_count)?;
    if let Some(port) = body.port {
        validate_port(port)?;
    }

    let container_id = Uuid::new_v4();
    let version_id = Uuid::new_v4();

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let environment = get_environment(
        tx,
        body.environment_id,
        organization_id,
        Some(body.project_id),
    )
    .await?;
    let registry =
        selected_external_registry(tx, organization_id, body.external_registry_id).await?;
    let resolved_image = images::resolve_image(&image, organization_id, registry.as_ref()).await?;
    let external_registry_id = registry.as_ref().map(|registry| registry.id);

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
        resolved_image: Set(resolved_image),
        public: Set(body.public),
        replica_count: Set(body.replica_count),
        port: Set(body.port),
        env: Set(body.env.clone()),
        resources: Set(body.resources.clone()),
        external_registry_id: Set(external_registry_id),
        health_check: Set(body.health_check.clone()),
        created_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;

    let mut pins = get_environment_timeline_pins(tx, &environment).await?;
    pins.set_container(container_id, version_id);
    let revision = revisions::create_revision(
        tx,
        &environment,
        &pins,
        Some(format!("Created container '{}'", name)),
        body.auto_deploy,
    )
    .await?;
    events::record(tx, organization_id, body.project_id, "container:created", serde_json::json!({"summary": format!("Created container '{}'", name), "target_id": container_id.to_string(), "environment_id": environment.id.to_string()}), auth.actor_id).await?;

    scoped.commit().await?;
    if body.auto_deploy {
        agent::emit_compute(
            environment.project_id,
            organization_id,
            environment.id,
            revision.id,
        )
        .await?;
    }

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
        ("environment_id" = Option<Uuid>, Query, description = "Filter by environment"),
        ("timeline_id" = Option<Uuid>, Query, description = "Revision whose pinned containers to return"),
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

    let environment = if let Some(environment_id) = query.environment_id {
        Some(get_environment(tx, environment_id, organization_id, query.project_id).await?)
    } else if let Some(project_id) = query.project_id {
        Some(find_main_environment_containers(tx, project_id, organization_id).await?)
    } else {
        None
    };

    if let Some(environment) = environment {
        let pins = if let Some(timeline_id) = query.timeline_id {
            get_project_revision_pins(tx, environment.project_id, timeline_id).await?
        } else {
            get_environment_timeline_pins(tx, &environment).await?
        };

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
        .filter(container::Column::OrganizationId.eq(organization_id))
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

async fn find_main_environment_containers(
    tx: &impl sea_orm::ConnectionTrait,
    project_id: Uuid,
    organization_id: Uuid,
) -> Result<project_environment::Model, AppError> {
    project_environment::Entity::find()
        .filter(project_environment::Column::ProjectId.eq(project_id))
        .filter(project_environment::Column::OrganizationId.eq(organization_id))
        .filter(project_environment::Column::Name.eq("main"))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Main environment not found for project".into()))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/containers/{container_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("container_id" = Uuid, Path, description = "Container ID"),
        ("environment_id" = Option<Uuid>, Query, description = "Environment that owns the revision history"),
        ("timeline_id" = Option<Uuid>, Query, description = "Revision whose pinned container version to return"),
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
    axum::extract::Query(query): axum::extract::Query<ContainerDetailQuery>,
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

    let version = if let Some(timeline_id) = query.timeline_id {
        let environment_id = query.environment_id.ok_or_else(|| {
            AppError::BadRequest("environment_id is required with timeline_id".into())
        })?;
        let environment =
            get_environment(tx, environment_id, organization_id, Some(c.project_id)).await?;
        let pins = get_project_revision_pins(tx, environment.project_id, timeline_id).await?;
        let version_id = pins.container.get(&c.id).ok_or_else(|| {
            AppError::NotFound("Container not present in timeline revision".into())
        })?;
        container_version::Entity::find_by_id(*version_id)
            .filter(container_version::Column::ContainerId.eq(c.id))
            .one(tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Container version not found".into()))?
    } else {
        container_version::Entity::find()
            .filter(container_version::Column::ContainerId.eq(c.id))
            .order_by_desc(container_version::Column::Version)
            .one(tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Container version not found".into()))?
    };

    scoped.commit().await?;

    Ok(Json(ContainerResponse {
        id: c.id,
        organization_id: c.organization_id,
        name: c.name,
        current_version: Some(resolve_latest_version(&version)),
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
        ("environment_id" = Uuid, Query, description = "Environment ID for the revision"),
        ("timeline_id" = Uuid, Query, description = "Revision that supplies the container update base"),
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
    axum::extract::Query(action): axum::extract::Query<UpdateContainerQuery>,
    Json(body): Json<UpdateContainerRequest>,
) -> Result<Json<ContainerResponse>, AppError> {
    update_container_with_options(
        tenant_db,
        auth,
        organization_id,
        container_id,
        action,
        body,
        false,
        "Updated container configuration",
    )
    .await
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/containers/{container_id}/deploy",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("container_id" = Uuid, Path, description = "Container ID"),
        ("environment_id" = Uuid, Query, description = "Environment ID for the revision"),
        ("timeline_id" = Uuid, Query, description = "Draft revision to deploy"),
    ),
    responses(
        (status = 200, description = "Container redeployed", body = ContainerResponse),
        (status = 400, description = "Container image is not refreshable"),
        (status = 404, description = "Not found"),
    ),
    tag = "containers",
)]
pub async fn redeploy_container(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, container_id)): Path<(Uuid, Uuid)>,
    axum::extract::Query(action): axum::extract::Query<UpdateContainerQuery>,
) -> Result<Json<ContainerResponse>, AppError> {
    update_container_with_options(
        tenant_db,
        auth,
        organization_id,
        container_id,
        action,
        UpdateContainerRequest {
            auto_deploy: true,
            ..Default::default()
        },
        true,
        "Redeployed latest container image",
    )
    .await
}

async fn update_container_with_options(
    tenant_db: TenantDatabase,
    auth: RequestAuthContext,
    organization_id: Uuid,
    container_id: Uuid,
    action: UpdateContainerQuery,
    body: UpdateContainerRequest,
    force_version: bool,
    timeline_summary: &str,
) -> Result<Json<ContainerResponse>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let mut c = container::Entity::find()
        .filter(container::Column::Id.eq(container_id))
        .filter(container::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Container not found".into()))?;

    let environment = get_environment(
        tx,
        action.environment_id,
        organization_id,
        Some(c.project_id),
    )
    .await?;

    if action.timeline_id != environment.draft_timeline {
        return Err(AppError::Conflict(
            "Fork this revision before changing its configuration".into(),
        ));
    }

    if let Some(ref new_name) = body.name {
        let trimmed = new_name.trim().to_string();
        if trimmed.is_empty() {
            return Err(AppError::BadRequest("Name is required".into()));
        }
        let mut active: container::ActiveModel = c.clone().into();
        active.name = Set(trimmed);
        active.updated_at = Set(Utc::now().fixed_offset());
        c = active.update(tx).await?;
    }

    if let Some(replica_count) = body.replica_count {
        validate_replica_count(replica_count)?;
    }
    if let Some(port) = body.port {
        validate_port(port)?;
    }

    let mut new_version: Option<container_version::Model> = None;
    let mut compute_revision = None;

    if has_config_change(&body) || force_version {
        let mut pins =
            get_project_revision_pins(tx, environment.project_id, action.timeline_id).await?;
        let base_version_id = pins.container.get(&container_id).ok_or_else(|| {
            AppError::NotFound("Container not present in timeline revision".into())
        })?;
        let base = container_version::Entity::find_by_id(*base_version_id)
            .filter(container_version::Column::ContainerId.eq(container_id))
            .one(tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Container version not found".into()))?;
        let latest_version_number = container_version::Entity::find()
            .filter(container_version::Column::ContainerId.eq(container_id))
            .order_by_desc(container_version::Column::Version)
            .one(tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Container version not found".into()))?;

        let next_image = body
            .image
            .as_deref()
            .map(str::trim)
            .unwrap_or(&base.image)
            .to_string();
        if next_image.is_empty() {
            return Err(AppError::BadRequest("Image is required".into()));
        }
        if force_version && !is_exact_latest_image(&next_image) {
            return Err(AppError::BadRequest(
                "Only images configured with the exact :latest tag can be redeployed".into(),
            ));
        }
        let next_registry_id = body
            .external_registry_id
            .unwrap_or(base.external_registry_id);
        let registry = selected_external_registry(tx, organization_id, next_registry_id).await?;
        let resolved_image =
            if body.image.is_some() || body.external_registry_id.is_some() || force_version {
                images::resolve_image(&next_image, organization_id, registry.as_ref()).await?
            } else {
                base.resolved_image.clone()
            };
        let external_registry_id = registry.as_ref().map(|registry| registry.id);

        let next_ver = latest_version_number.version + 1;
        let version_id = Uuid::new_v4();

        let cv = container_version::ActiveModel {
            id: Set(version_id),
            container_id: Set(container_id),
            organization_id: Set(organization_id),
            version: Set(next_ver),
            image: Set(next_image),
            resolved_image: Set(resolved_image),
            public: Set(body.public.unwrap_or(base.public)),
            replica_count: Set(body.replica_count.unwrap_or(base.replica_count)),
            port: Set(body.port.or(base.port)),
            env: Set(body.env.clone().or(base.env.clone())),
            resources: Set(body.resources.clone().or(base.resources.clone())),
            external_registry_id: Set(external_registry_id),
            health_check: Set(body.health_check.clone().or(base.health_check.clone())),
            created_at: Set(Utc::now().fixed_offset()),
        };

        new_version = Some(cv.insert(tx).await?);

        pins.set_container(container_id, version_id);
        let revision = revisions::create_revision(
            tx,
            &environment,
            &pins,
            Some(timeline_summary.into()),
            body.auto_deploy,
        )
        .await?;
        if body.auto_deploy {
            compute_revision = Some(revision.id);
        }
    }

    events::record(tx, organization_id, c.project_id, "container:updated", serde_json::json!({"summary": format!("Updated container '{}'", c.name), "target_id": container_id.to_string(), "environment_id": environment.id.to_string()}), auth.actor_id).await?;

    let latest_version = match new_version {
        Some(v) => Some(v),
        None => {
            container_version::Entity::find()
                .filter(container_version::Column::ContainerId.eq(container_id))
                .order_by_desc(container_version::Column::Version)
                .one(tx)
                .await?
        }
    };

    scoped.commit().await?;
    if let Some(revision_id) = compute_revision {
        agent::emit_compute(
            environment.project_id,
            organization_id,
            environment.id,
            revision_id,
        )
        .await?;
    }

    Ok(Json(ContainerResponse {
        id: c.id,
        organization_id: c.organization_id,
        name: c.name,
        current_version: latest_version.map(|v| resolve_latest_version(&v)),
        project_id: Some(c.project_id),
        region_id: c.region_id,
        created_at: c.created_at.to_string(),
        updated_at: c.updated_at.to_string(),
    }))
}

#[cfg(test)]
mod redeploy_tests {
    use super::{is_exact_latest_image, validate_port, validate_replica_count};

    #[test]
    fn only_exact_latest_images_can_be_redeployed() {
        assert!(is_exact_latest_image("nginx:latest"));
        assert!(is_exact_latest_image("registry.example.com/app:latest"));
        assert!(!is_exact_latest_image("nginx:Latest"));
        assert!(!is_exact_latest_image("nginx:latest@sha256:abc"));
    }

    #[test]
    fn replica_and_port_ranges_are_enforced() {
        assert!(validate_replica_count(1).is_ok());
        assert!(validate_replica_count(0).is_err());
        assert!(validate_replica_count(-3).is_err());
        assert!(validate_replica_count(101).is_err());

        assert!(validate_port(80).is_ok());
        assert!(validate_port(65535).is_ok());
        assert!(validate_port(0).is_err());
        assert!(validate_port(-80).is_err());
        assert!(validate_port(65536).is_err());
    }
}

#[utoipa::path(
    delete,
    path = "/api/organization/{organization_id}/containers/{container_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("container_id" = Uuid, Path, description = "Container ID"),
        ("environment_id" = Uuid, Query, description = "Environment ID for the revision"),
        ("timeline_id" = Uuid, Query, description = "Draft revision to remove the container from"),
        ("deploy" = Option<bool>, Query, description = "Deploy the removal immediately"),
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

    let environment = get_environment(
        tx,
        action.environment_id,
        organization_id,
        Some(c.project_id),
    )
    .await?;

    if action.timeline_id != environment.draft_timeline {
        return Err(AppError::Conflict(
            "Switch to the draft revision before removing a container".into(),
        ));
    }

    let mut pins =
        get_project_revision_pins(tx, environment.project_id, action.timeline_id).await?;
    pins.remove_container(&container_id);
    let revision = revisions::create_revision(
        tx,
        &environment,
        &pins,
        Some(format!("Removed container '{}'", c.name)),
        action.deploy,
    )
    .await?;
    events::record(tx, organization_id, c.project_id, "container:removed", serde_json::json!({"summary": format!("Removed container '{}'", c.name), "target_id": container_id.to_string(), "environment_id": environment.id.to_string()}), auth.actor_id).await?;

    scoped.commit().await?;
    if action.deploy {
        agent::emit_compute(
            environment.project_id,
            organization_id,
            environment.id,
            revision.id,
        )
        .await?;
    }

    Ok(Json(serde_json::json!({ "success": true })))
}
