pub mod history;

use axum::{Json, extract::Path};
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::auth::{AuthContext, RequestAuthContext};
use crate::models::entities::container;
use crate::models::manifest::{ContainerConfig, RevisionManifest};
use crate::state::{TenantDatabase, get_app_state};
use lib::services::containers as container_service;

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
    let state = get_app_state();
    let created = lib::services::containers::create(
        &tenant_db,
        state.identity_db.connection(),
        &state.secrets,
        state.config.registry_token_ttl_seconds,
        organization_id,
        auth.actor_id,
        lib::services::containers::CreateInput {
            project_id: body.project_id,
            environment_id: body.environment_id,
            name: body.name,
            image: body.image,
            public: body.public,
            replica_count: body.replica_count,
            port: body.port,
            env: body.env,
            cpu: body.cpu,
            memory: body.memory,
            external_registry_id: body.external_registry_id,
            health_check: body.health_check,
            auto_deploy: body.auto_deploy,
            region_id: body.region_id,
        },
    )
    .await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(build_response(
            &created.container,
            &created.manifest,
            created.revision_id,
        )),
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
    let rows = container_service::list(
        &tenant_db,
        organization_id,
        container_service::ListInput {
            project_id: query.project_id,
            environment_id: query.environment_id,
            timeline_id: query.timeline_id,
        },
    )
    .await?;
    Ok(Json(
        rows.iter()
            .map(|row| build_response(&row.container, &row.manifest, row.revision_id))
            .collect(),
    ))
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
    let row = container_service::get(
        &tenant_db,
        organization_id,
        container_service::GetInput {
            container_id,
            environment_id: query.environment_id,
            timeline_id: query.timeline_id,
        },
    )
    .await?;
    Ok(Json(build_response(
        &row.container,
        &row.manifest,
        row.revision_id,
    )))
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
    let state = get_app_state();
    let updated = container_service::update(
        &tenant_db,
        state.identity_db.connection(),
        &state.secrets,
        state.config.registry_token_ttl_seconds,
        organization_id,
        auth.actor_id,
        container_service::UpdateInput {
            container_id,
            environment_id: action.environment_id,
            timeline_id: action.timeline_id,
            name: body.name,
            image: body.image,
            public: body.public,
            replica_count: body.replica_count,
            port: body.port,
            env: body.env,
            cpu: body.cpu,
            memory: body.memory,
            external_registry_id: body.external_registry_id,
            health_check: body.health_check,
            auto_deploy: body.auto_deploy,
            redeploy,
            timeline_summary: timeline_summary.into(),
        },
    )
    .await?;
    Ok(Json(build_response(
        &updated.container,
        &updated.manifest,
        updated.revision_id,
    )))
}

#[cfg(test)]
mod redeploy_tests {
    use super::UpdateContainerRequest;
    use lib::services::containers::{validate_port, validate_replica_count, validate_resources};

    #[test]
    fn update_resources_distinguish_omitted_and_null() {
        let omitted: UpdateContainerRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(omitted.cpu, None);
        assert_eq!(omitted.memory, None);
        let cleared: UpdateContainerRequest =
            serde_json::from_str(r#"{"cpu":null,"memory":null}"#).unwrap();
        assert_eq!(cleared.cpu, Some(None));
        assert_eq!(cleared.memory, Some(None));
    }

    #[test]
    fn replica_port_and_resource_validation() {
        assert!(validate_replica_count(1).is_ok());
        assert!(validate_replica_count(0).is_err());
        assert!(validate_port(80).is_ok());
        assert!(validate_port(0).is_err());
        assert!(validate_resources(&Some("0.5".into()), &Some("1024Mi".into())).is_ok());
        assert!(validate_resources(&Some("big".into()), &Some("1024Mi".into())).is_err());
        assert!(validate_resources(&Some("0.5".into()), &None).is_err());
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
    container_service::delete(
        &tenant_db,
        organization_id,
        auth.actor_id,
        container_service::DeleteInput {
            container_id,
            environment_id: action.environment_id,
            timeline_id: action.timeline_id,
            deploy: action.deploy,
        },
    )
    .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}
