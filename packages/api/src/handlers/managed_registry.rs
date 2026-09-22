use axum::{
    Json,
    extract::{Path, Query},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError,
    middleware::auth::AuthContext,
    models::entities::managed_registry,
    state::get_app_state,
    utils::pagination::{PaginatedResponse, PaginationQuery},
};
use lib::entities::managed_registry::ManagedRegistryStatus;
use lib::services::managed_registry as managed_registry_service;

const REGISTRY_BUCKET_NAME: &str = "registry";

#[derive(Deserialize, ToSchema)]
pub struct ActivateManagedRegistryRequest {
    pub region_id: Uuid,
}

#[derive(Deserialize, ToSchema)]
pub struct ListRegistryGarbageCollectionQuery {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct RegistryGcJobResponse {
    pub id: Uuid,
    pub status: String,
    pub trigger: String,
    pub available_at: String,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct RegistryGcRunResponse {
    pub started_at: String,
    pub finished_at: String,
    pub bytes_before: Option<i64>,
    pub bytes_after: Option<i64>,
    pub result: String,
    pub error: Option<String>,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct RegistryGarbageCollectionResponse {
    pub active_job: Option<RegistryGcJobResponse>,
    pub gc_runs: PaginatedResponse<RegistryGcRunResponse>,
}

#[derive(Serialize, ToSchema)]
pub struct ManagedRegistryResponse {
    pub organization_id: Uuid,
    pub region_id: Uuid,
    pub status: String,
    pub storage_revision: Uuid,
    pub created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct ResolvedManagedRegistry {
    pub organization_id: Uuid,
    pub organization_slug: String,
    pub storage_revision: Uuid,
    pub status: String,
    pub access_key_id: String,
    #[schema(ignore)]
    pub secret_access_key: String,
    pub bucket_name: String,
    pub storage_endpoint_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct ResolveManagedRegistryQuery {
    pub repository_name: Option<String>,
    pub repository_id: Option<Uuid>,
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/registry",
    params(("organization_id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Managed Registry configuration", body = ManagedRegistryResponse),
        (status = 404, description = "Managed Registry is not activated"),
    ),
    tag = "registry",
)]
pub async fn get_registry(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
) -> Result<Json<ManagedRegistryResponse>, AppError> {
    let view = managed_registry_service::get(&tenant_db, organization_id).await?;
    Ok(Json(response(&view.registry, view.region_id)))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/registry/garbage-collection",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("page" = Option<u64>, Query, description = "Page number"),
        ("per_page" = Option<u64>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "Garbage-collection status with paginated runs", body = RegistryGarbageCollectionResponse),
        (status = 404, description = "Managed Registry is not activated"),
    ),
    tag = "registry",
)]
pub async fn get_garbage_collection(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
    Query(query): Query<ListRegistryGarbageCollectionQuery>,
) -> Result<Json<RegistryGarbageCollectionResponse>, AppError> {
    let pagination = PaginationQuery {
        page: query.page,
        per_page: query.per_page,
    };
    let view = managed_registry_service::garbage_collection(
        &tenant_db,
        organization_id,
        pagination.page(),
        pagination.per_page(),
    )
    .await?;
    return Ok(Json(gc_view_response(view)));
}

#[utoipa::path(
    put,
    path = "/api/organization/{organization_id}/registry",
    request_body = ActivateManagedRegistryRequest,
    params(("organization_id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 201, description = "Managed Registry activated", body = ManagedRegistryResponse),
        (status = 200, description = "Managed Registry already active", body = ManagedRegistryResponse),
        (status = 404, description = "Organization or active region not found"),
        (status = 409, description = "Active region has no S3 provider or registry creation is reserved (in progress or awaiting recovery)"),
    ),
    tag = "registry",
)]
pub async fn activate_registry(
    AuthContext { tenant_db, auth }: AuthContext,
    Path(organization_id): Path<Uuid>,
    Json(body): Json<ActivateManagedRegistryRequest>,
) -> Result<(StatusCode, Json<ManagedRegistryResponse>), AppError> {
    let state = get_app_state();
    let activated = managed_registry_service::activate(
        &tenant_db,
        &state.s3_providers,
        &state.secrets,
        organization_id,
        auth.actor_id,
        body.region_id,
    )
    .await?;
    let status = if activated.created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((
        status,
        Json(response(&activated.registry, activated.region_id)),
    ))
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/registry/garbage-collection",
    params(("organization_id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 202, description = "Garbage collection queued", body = RegistryGarbageCollectionResponse),
        (status = 404, description = "Managed Registry is not activated"),
        (status = 409, description = "Garbage collection is already running"),
    ),
    tag = "registry",
)]
pub async fn run_garbage_collection(
    AuthContext { tenant_db, auth }: AuthContext,
    Path(organization_id): Path<Uuid>,
) -> Result<(StatusCode, Json<RegistryGarbageCollectionResponse>), AppError> {
    let view = managed_registry_service::queue_garbage_collection(
        &tenant_db,
        organization_id,
        auth.actor_id,
    )
    .await?;
    return Ok((StatusCode::ACCEPTED, Json(gc_view_response(view))));
}

#[utoipa::path(
    get,
    path = "/internal/organizations/{organization_id}/registry",
    params(("organization_id" = Uuid, Path)),
    responses(
        (status = 200, body = ResolvedManagedRegistry),
        (status = 404, body = crate::errors::ErrorResponse),
    ),
    security(("serviceToken" = [])),
    tag = "internal",
)]
pub async fn resolve_registry(
    Path(organization_id): Path<Uuid>,
    Query(query): Query<ResolveManagedRegistryQuery>,
) -> Result<Json<ResolvedManagedRegistry>, AppError> {
    let state = get_app_state();
    let resolved = managed_registry_service::resolve(
        state.identity_db.connection(),
        &state.secrets,
        organization_id,
        query.repository_name,
        query.repository_id,
    )
    .await?;
    Ok(Json(ResolvedManagedRegistry {
        organization_id: resolved.organization_id,
        organization_slug: resolved.organization_slug,
        storage_revision: resolved.storage_revision,
        status: resolved.status,
        access_key_id: resolved.access_key_id,
        secret_access_key: resolved.secret_access_key,
        bucket_name: REGISTRY_BUCKET_NAME.into(),
        storage_endpoint_url: state.config.storage_internal_url,
        repository_name: resolved.repository_name,
        repository_id: resolved.repository_id,
    }))
}

fn response(registry: &managed_registry::Model, region_id: Uuid) -> ManagedRegistryResponse {
    ManagedRegistryResponse {
        organization_id: registry.organization_id,
        region_id,
        status: status_name(&registry.status).into(),
        storage_revision: registry.storage_revision,
        created_at: registry.created_at.to_rfc3339(),
    }
}

fn gc_view_response(
    view: managed_registry_service::GarbageCollectionView,
) -> RegistryGarbageCollectionResponse {
    let active_job = view.active_job.map(|job| RegistryGcJobResponse {
        id: job.id,
        status: job.status,
        trigger: job.trigger,
        available_at: job.available_at.to_rfc3339(),
    });
    let runs = view
        .runs
        .into_iter()
        .map(|run| RegistryGcRunResponse {
            started_at: run.started_at.to_rfc3339(),
            finished_at: run.finished_at.to_rfc3339(),
            bytes_before: run.bytes_before,
            bytes_after: run.bytes_after,
            result: run.result,
            error: run.error,
        })
        .collect();
    RegistryGarbageCollectionResponse {
        active_job,
        gc_runs: PaginatedResponse::new(runs, view.total, view.page, view.per_page),
    }
}

fn status_name(status: &ManagedRegistryStatus) -> &'static str {
    match status {
        ManagedRegistryStatus::Active => "active",
        ManagedRegistryStatus::Maintenance => "maintenance",
    }
}
