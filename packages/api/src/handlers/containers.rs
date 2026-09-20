pub mod history;

use axum::{Json, extract::Path};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter,
    QueryOrder, Set,
};
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{
    databases::{validate_cpu, validate_ram, verify_org_access},
    external_registries::find_registry,
};
use crate::errors::AppError;
use crate::middleware::auth::{AuthContext, RequestAuthContext};
use crate::models::entities::{container, project, project_environment, project_timeline};
use crate::models::manifest::{ContainerConfig, RevisionManifest};
use crate::state::{TenantDatabase, get_app_state};
use lib::services::{agent, events, images, revisions};

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
    pub cpu: Option<String>,
    pub memory: Option<String>,
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
    #[serde(default, deserialize_with = "deserialize_present_nullable")]
    pub cpu: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_present_nullable")]
    pub memory: Option<Option<String>>,
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
    pub configuration: Option<ContainerConfig>,
    pub revision_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub region_id: Uuid,
    pub created_at: String,
    pub updated_at: String,
}

fn has_config_change(req: &UpdateContainerRequest) -> bool {
    req.image.is_some()
        || req.public.is_some()
        || req.replica_count.is_some()
        || req.port.is_some()
        || req.env.is_some()
        || req.cpu.is_some()
        || req.memory.is_some()
        || req.external_registry_id.is_some()
        || req.health_check.is_some()
}

fn image_resolution_needed(
    request: &UpdateContainerRequest,
    base: &ContainerConfig,
    redeploy: bool,
) -> bool {
    let image_changed = request
        .image
        .as_deref()
        .is_some_and(|image| image.trim() != base.image);
    let registry_changed = request
        .external_registry_id
        .as_ref()
        .is_some_and(|registry_id| *registry_id != base.external_registry_id);
    image_changed || registry_changed || redeploy
}

fn next_external_registry_id(
    request: &UpdateContainerRequest,
    base: &ContainerConfig,
) -> Option<Uuid> {
    request
        .external_registry_id
        .unwrap_or(base.external_registry_id)
}

pub(crate) fn validate_replica_count(replica_count: i32) -> Result<(), AppError> {
    if !(1..=100).contains(&replica_count) {
        return Err(AppError::BadRequest(
            "Replica count must be between 1 and 100".into(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_container_resources(
    cpu: &Option<String>,
    memory: &Option<String>,
) -> Result<(), AppError> {
    match (cpu, memory) {
        (None, None) => Ok(()),
        (Some(cpu), Some(memory)) => {
            validate_cpu(cpu)?;
            validate_ram(memory)?;
            Ok(())
        }
        _ => Err(AppError::BadRequest(
            "Resources require both cpu and memory".into(),
        )),
    }
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
    tx: &impl ConnectionTrait,
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
    tx: &impl ConnectionTrait,
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

async fn resolve_environment(
    tx: &impl ConnectionTrait,
    organization_id: Uuid,
    project_id: Uuid,
    environment_id: Option<Uuid>,
) -> Result<project_environment::Model, AppError> {
    if let Some(environment_id) = environment_id {
        return get_environment(tx, environment_id, organization_id, Some(project_id)).await;
    }
    let default_id = project::Entity::find_by_id(project_id)
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .and_then(|project| project.default_environment_id)
        .ok_or_else(|| AppError::NotFound("Project has no default environment".into()))?;
    get_environment(tx, default_id, organization_id, Some(project_id)).await
}

async fn timeline_manifest(
    tx: &impl ConnectionTrait,
    timeline_id: Uuid,
    organization_id: Uuid,
    project_id: Option<Uuid>,
) -> Result<(project_timeline::Model, RevisionManifest), AppError> {
    let mut query = project_timeline::Entity::find()
        .filter(project_timeline::Column::Id.eq(timeline_id))
        .filter(project_timeline::Column::OrganizationId.eq(organization_id));
    if let Some(project_id) = project_id {
        query = query.filter(project_timeline::Column::ProjectId.eq(project_id));
    }
    let timeline = query
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Timeline revision is not in this project".into()))?;
    let manifest = revisions::load_manifest(tx, timeline.manifest_id).await?;
    Ok((timeline, manifest))
}

async fn draft_manifest(
    tx: &impl ConnectionTrait,
    environment: &project_environment::Model,
) -> Result<(project_timeline::Model, RevisionManifest), AppError> {
    timeline_manifest(
        tx,
        environment.draft_timeline,
        environment.organization_id,
        Some(environment.project_id),
    )
    .await
}

fn build_response(
    container: &container::Model,
    manifest: &RevisionManifest,
    revision_id: Uuid,
) -> ContainerResponse {
    ContainerResponse {
        id: container.id,
        organization_id: container.organization_id,
        name: manifest
            .containers
            .get(&container.id)
            .map(|config| config.name.clone())
            .unwrap_or_else(|| container.name.clone()),
        configuration: manifest.containers.get(&container.id).cloned(),
        revision_id: Some(revision_id),
        project_id: Some(container.project_id),
        region_id: manifest
            .containers
            .get(&container.id)
            .map(|config| config.region_id)
            .unwrap_or(container.region_id),
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
    validate_container_resources(&body.cpu, &body.memory)?;

    let container_id = Uuid::new_v4();

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let state = get_app_state();
    let image_context = images::ImageContext {
        identity_db: state.identity_db.connection(),
        secrets: &state.secrets,
        registry_token_ttl_seconds: state.config.registry_token_ttl_seconds,
    };

    let environment = get_environment(
        tx,
        body.environment_id,
        organization_id,
        Some(body.project_id),
    )
    .await?;
    let registry =
        selected_external_registry(tx, organization_id, body.external_registry_id).await?;
    let resolved_image = images::resolve_image(
        &image,
        organization_id,
        registry.as_ref(),
        Some(&image_context),
    )
    .await?;
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

    let (_, mut manifest) = draft_manifest(tx, &environment).await?;
    manifest.containers.insert(
        container_id,
        ContainerConfig {
            name: name.clone(),
            region_id: body.region_id,
            image,
            resolved_image,
            external_registry_id,
            replica_count: body.replica_count,
            port: body.port,
            public: body.public,
            cpu: body.cpu.clone(),
            memory: body.memory.clone(),
            health_check: body.health_check.clone(),
            env: body.env.clone(),
        },
    );
    let revision = revisions::create_revision(
        tx,
        &environment,
        &manifest,
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
        Json(build_response(&created_container, &manifest, revision.id)),
    ))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/containers",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Option<Uuid>, Query, description = "Filter by project"),
        ("environment_id" = Option<Uuid>, Query, description = "Filter by environment"),
        ("timeline_id" = Option<Uuid>, Query, description = "Revision whose manifest containers to return"),
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

    let mut requested_environment = None;
    let project_id = if let Some(environment_id) = query.environment_id {
        let environment =
            get_environment(tx, environment_id, organization_id, query.project_id).await?;
        let project_id = environment.project_id;
        requested_environment = Some(environment);
        Some(project_id)
    } else {
        match (query.project_id, query.timeline_id) {
            (Some(id), _) => Some(id),
            (None, Some(id)) => Some(
                timeline_manifest(tx, id, organization_id, None)
                    .await?
                    .0
                    .project_id,
            ),
            _ => None,
        }
    };

    if let Some(project_id) = project_id {
        let environment = match requested_environment {
            Some(environment) => environment,
            None => resolve_environment(tx, organization_id, project_id, None).await?,
        };
        let timeline_id = query.timeline_id.unwrap_or(environment.draft_timeline);
        let (_, manifest) =
            timeline_manifest(tx, timeline_id, organization_id, Some(project_id)).await?;

        let responses = containers_for_manifest(tx, &manifest, timeline_id).await?;
        scoped.commit().await?;
        return Ok(Json(responses));
    }

    #[derive(FromQueryResult)]
    struct DefaultManifest {
        project_id: Uuid,
        revision_id: Uuid,
        schema_version: i32,
        configuration: serde_json::Value,
    }
    let defaults = DefaultManifest::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT p.id AS project_id, t.id AS revision_id, m.schema_version, m.configuration
         FROM project p
         JOIN project_environment e ON e.id = p.default_environment_id AND e.project_id = p.id
         JOIN project_timeline t ON t.id = e.draft_timeline AND t.project_id = p.id
         JOIN project_revision_manifest m ON m.id = t.manifest_id
         WHERE p.organization_id = $1",
        [organization_id.into()],
    ))
    .all(tx)
    .await?;

    let containers = container::Entity::find()
        .filter(container::Column::OrganizationId.eq(organization_id))
        .all(tx)
        .await?;
    let by_id: std::collections::HashMap<Uuid, &container::Model> =
        containers.iter().map(|c| (c.id, c)).collect();

    let mut responses = Vec::new();
    for row in defaults {
        let manifest = RevisionManifest::from_json_value(&row.configuration, row.schema_version)?;
        for container_id in manifest.containers.keys() {
            if let Some(container) = by_id.get(container_id) {
                if container.project_id == row.project_id {
                    responses.push(build_response(container, &manifest, row.revision_id));
                }
            }
        }
    }
    responses.sort_by(|a, b| a.name.cmp(&b.name));

    scoped.commit().await?;
    Ok(Json(responses))
}

async fn containers_for_manifest(
    tx: &impl ConnectionTrait,
    manifest: &RevisionManifest,
    revision_id: Uuid,
) -> Result<Vec<ContainerResponse>, AppError> {
    if manifest.containers.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<Uuid> = manifest.containers.keys().copied().collect();
    let containers = container::Entity::find()
        .filter(container::Column::Id.is_in(ids))
        .order_by_asc(container::Column::Name)
        .all(tx)
        .await?;
    let mut responses: Vec<_> = containers
        .iter()
        .filter(|container| manifest.containers.contains_key(&container.id))
        .map(|container| build_response(container, manifest, revision_id))
        .collect();
    responses.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    Ok(responses)
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/containers/{container_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("container_id" = Uuid, Path, description = "Container ID"),
        ("environment_id" = Option<Uuid>, Query, description = "Environment whose draft or revision to read"),
        ("timeline_id" = Option<Uuid>, Query, description = "Revision whose manifest container to return"),
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

    let timeline_id = if let Some(timeline_id) = query.timeline_id {
        if query.environment_id.is_none() {
            return Err(AppError::BadRequest(
                "environment_id is required with timeline_id".into(),
            ));
        }
        get_environment(
            tx,
            query.environment_id.unwrap(),
            organization_id,
            Some(c.project_id),
        )
        .await?;
        timeline_id
    } else {
        resolve_environment(tx, organization_id, c.project_id, query.environment_id)
            .await?
            .draft_timeline
    };

    let (timeline, manifest) =
        timeline_manifest(tx, timeline_id, organization_id, Some(c.project_id)).await?;
    if !manifest.containers.contains_key(&c.id) {
        return Err(AppError::NotFound(
            "Container not present in timeline revision".into(),
        ));
    }

    scoped.commit().await?;
    Ok(Json(build_response(&c, &manifest, timeline.id)))
}

#[utoipa::path(
    patch,
    path = "/api/organization/{organization_id}/containers/{container_id}",
    request_body = UpdateContainerRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("container_id" = Uuid, Path, description = "Container ID"),
        ("environment_id" = Uuid, Query, description = "Environment ID for the revision"),
        ("timeline_id" = Uuid, Query, description = "Draft revision that supplies the update base"),
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
        ("timeline_id" = Uuid, Query, description = "Draft revision to redeploy"),
    ),
    responses(
        (status = 200, description = "Container redeployed", body = ContainerResponse),
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
    redeploy: bool,
    timeline_summary: &str,
) -> Result<Json<ContainerResponse>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let state = get_app_state();
    let image_context = images::ImageContext {
        identity_db: state.identity_db.connection(),
        secrets: &state.secrets,
        registry_token_ttl_seconds: state.config.registry_token_ttl_seconds,
    };

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
            "Select the environment's draft revision before changing its configuration".into(),
        ));
    }

    let next_name = match body.name.as_ref() {
        Some(new_name) => {
            let trimmed = new_name.trim().to_string();
            if trimmed.is_empty() {
                return Err(AppError::BadRequest("Name is required".into()));
            }
            Some(trimmed)
        }
        None => None,
    };

    if let Some(replica_count) = body.replica_count {
        validate_replica_count(replica_count)?;
    }
    if let Some(port) = body.port {
        validate_port(port)?;
    }

    let (draft_timeline, manifest) =
        timeline_manifest(tx, action.timeline_id, organization_id, Some(c.project_id)).await?;
    let base = manifest
        .containers
        .get(&container_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound("Container not present in timeline revision".into()))?;

    let should_revise = has_config_change(&body) || next_name.is_some() || redeploy;
    let mut compute_revision = None;
    let mut response_manifest = manifest.clone();
    let mut response_revision = draft_timeline.id;

    if should_revise {
        let next_cpu = body.cpu.clone().unwrap_or_else(|| base.cpu.clone());
        let next_memory = body.memory.clone().unwrap_or_else(|| base.memory.clone());
        validate_container_resources(&next_cpu, &next_memory)?;

        let next_image = body
            .image
            .as_deref()
            .map(str::trim)
            .unwrap_or(&base.image)
            .to_string();
        if next_image.is_empty() {
            return Err(AppError::BadRequest("Image is required".into()));
        }
        let next_registry_id = next_external_registry_id(&body, &base);
        let needs_image_resolution = image_resolution_needed(&body, &base, redeploy);
        let (resolved_image, external_registry_id) = if needs_image_resolution {
            let registry =
                selected_external_registry(tx, organization_id, next_registry_id).await?;
            (
                images::resolve_image(
                    &next_image,
                    organization_id,
                    registry.as_ref(),
                    Some(&image_context),
                )
                .await?,
                registry.as_ref().map(|registry| registry.id),
            )
        } else {
            (base.resolved_image.clone(), base.external_registry_id)
        };

        let next_config = ContainerConfig {
            name: next_name.unwrap_or_else(|| base.name.clone()),
            region_id: base.region_id,
            image: next_image,
            resolved_image,
            external_registry_id,
            replica_count: body.replica_count.unwrap_or(base.replica_count),
            port: body.port.or(base.port),
            public: body.public.unwrap_or(base.public),
            cpu: next_cpu,
            memory: next_memory,
            health_check: body.health_check.clone().or(base.health_check.clone()),
            env: body.env.clone().or(base.env.clone()),
        };
        response_manifest
            .containers
            .insert(container_id, next_config);

        let revision = revisions::create_revision(
            tx,
            &environment,
            &response_manifest,
            Some(timeline_summary.into()),
            body.auto_deploy,
        )
        .await?;
        response_revision = revision.id;
        if body.auto_deploy {
            compute_revision = Some(revision.id);
        }
    }

    events::record(tx, organization_id, c.project_id, "container:updated", serde_json::json!({"summary": format!("Updated container '{}'", c.name), "target_id": container_id.to_string(), "environment_id": environment.id.to_string()}), auth.actor_id).await?;

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

    Ok(Json(build_response(
        &c,
        &response_manifest,
        response_revision,
    )))
}

#[cfg(test)]
mod redeploy_tests {
    use super::{
        ContainerConfig, UpdateContainerRequest, has_config_change, image_resolution_needed,
        next_external_registry_id, validate_container_resources, validate_port,
        validate_replica_count,
    };
    use uuid::Uuid;

    #[test]
    fn update_resources_distinguish_omitted_and_null() {
        let omitted: UpdateContainerRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(omitted.cpu, None);
        assert_eq!(omitted.memory, None);

        let cleared: UpdateContainerRequest =
            serde_json::from_str(r#"{"cpu":null,"memory":null}"#).unwrap();
        assert_eq!(cleared.cpu, Some(None));
        assert_eq!(cleared.memory, Some(None));
        assert!(has_config_change(&cleared));
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

    #[test]
    fn resources_require_valid_cpu_and_memory() {
        let cpu = |v: &str| Some(v.to_string());
        let memory = |v: &str| Some(v.to_string());
        assert!(validate_container_resources(&None, &None).is_ok());
        assert!(validate_container_resources(&cpu("0.5"), &memory("1024Mi")).is_ok());
        assert!(validate_container_resources(&cpu("0.5"), &None).is_err());
        assert!(validate_container_resources(&cpu("big"), &memory("1024Mi")).is_err());
        assert!(validate_container_resources(&cpu("0.5"), &memory("1GiB")).is_err());
    }

    #[test]
    fn unchanged_image_and_registry_do_not_resolve_again() {
        let base = ContainerConfig {
            name: "web".into(),
            region_id: Uuid::nil(),
            image: "web:latest".into(),
            resolved_image: "web@sha256:old".into(),
            external_registry_id: Some(Uuid::from_u128(1)),
            replica_count: 1,
            port: None,
            public: false,
            cpu: None,
            memory: None,
            health_check: None,
            env: None,
        };
        let unchanged = UpdateContainerRequest {
            image: Some("  web:latest  ".into()),
            external_registry_id: Some(Some(Uuid::from_u128(1))),
            replica_count: Some(2),
            ..Default::default()
        };
        assert!(!image_resolution_needed(&unchanged, &base, false));
        assert!(image_resolution_needed(&unchanged, &base, true));

        let cleared = UpdateContainerRequest {
            external_registry_id: Some(None),
            ..Default::default()
        };
        assert_eq!(next_external_registry_id(&cleared, &base), None);
        assert!(image_resolution_needed(&cleared, &base, false));
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

    let (_, mut manifest) = draft_manifest(tx, &environment).await?;
    manifest.containers.remove(&container_id);
    let revision = revisions::create_revision(
        tx,
        &environment,
        &manifest,
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
