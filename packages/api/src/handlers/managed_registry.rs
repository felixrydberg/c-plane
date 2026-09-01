use axum::{
    Json,
    extract::{Path, Query},
    http::StatusCode,
};
use chrono::{DateTime, FixedOffset};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseTransaction,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set, Statement,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError,
    middleware::auth::AuthContext,
    models::entities::{
        bucket, bucket_grant, credential, managed_registry, managed_registry_gc_run, region, secret,
    },
    services::buckets,
    state::get_app_state,
    utils::pagination::{PaginatedResponse, PaginationQuery},
};
use lib::entities::{
    managed_registry::ManagedRegistryStatus,
    region::{RegionRoutingMode, RegionStatus},
    secret::SecretScope,
};
use lib::operation::{Operation, registry_gc::RegistryGc};

use super::{databases::verify_org_access, registry_access_tokens::record_event};

const REGISTRY_BUCKET_NAME: &str = "registry";
const ACCESS_KEY_PREFIX: &str = "CP";

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
}

#[derive(Deserialize, Serialize)]
struct S3SecretKey {
    secret_access_key: String,
}

struct ActiveGcJob {
    id: Uuid,
    status: String,
    trigger: String,
    available_at: DateTime<FixedOffset>,
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
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let (registry, foundation) = managed_registry::Entity::find_by_id(organization_id)
        .find_also_related(bucket::Entity)
        .one(scoped.connection())
        .await?
        .ok_or_else(|| AppError::NotFound("Managed Registry is not activated".into()))?;
    let foundation =
        foundation.ok_or_else(|| AppError::NotFound("Managed Registry bucket not found".into()))?;
    let response = response(&registry, foundation.region_id);
    scoped.commit().await?;
    Ok(Json(response))
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
    verify_org_access(&tenant_db, organization_id)?;
    let pagination = PaginationQuery {
        page: query.page,
        per_page: query.per_page,
    };
    let page = pagination.page();
    let per_page = pagination.per_page();
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let registry = find_registry(scoped.connection(), organization_id).await?;
    let response =
        garbage_collection_response(scoped.connection(), &registry, page, per_page).await?;
    scoped.commit().await?;
    Ok(Json(response))
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
        (status = 409, description = "Active region has no S3 provider"),
    ),
    tag = "registry",
)]
pub async fn activate_registry(
    AuthContext { tenant_db, auth }: AuthContext,
    Path(organization_id): Path<Uuid>,
    Json(body): Json<ActivateManagedRegistryRequest>,
) -> Result<(StatusCode, Json<ManagedRegistryResponse>), AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let state = get_app_state();
    let providers = state.s3_providers;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    lock_organization(tx, organization_id).await?;

    if let Some((existing, foundation)) = managed_registry::Entity::find_by_id(organization_id)
        .find_also_related(bucket::Entity)
        .one(tx)
        .await?
    {
        let foundation = foundation
            .ok_or_else(|| AppError::NotFound("Managed Registry bucket not found".into()))?;
        let response = response(&existing, foundation.region_id);
        scoped.commit().await?;
        return Ok((StatusCode::OK, Json(response)));
    }

    let region = region::Entity::find_by_id(body.region_id)
        .filter(region::Column::Status.eq(RegionStatus::Active))
        .filter(region::Column::RoutingMode.ne(RegionRoutingMode::Disabled))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Active region not found".into()))?;
    let provider_id = region
        .s3_provider_id
        .ok_or_else(|| AppError::Conflict("Region has no S3 provider".into()))?;
    let foundation_bucket_id =
        buckets::create(tx, &providers, organization_id, region.id, provider_id).await?;
    let provisioned = provision_metadata(tx, organization_id, foundation_bucket_id).await;
    let registry = match provisioned {
        Ok(registry) => registry,
        Err(error) => {
            let _ = providers
                .delete_bucket(provider_id, foundation_bucket_id)
                .await;
            return Err(error);
        }
    };
    if let Err(error) = record_event(
        tx,
        organization_id,
        auth.actor_id,
        "managed-registry:activated",
        json!({ "summary": "Activated managed Registry", "target_id": organization_id }),
    )
    .await
    {
        let _ = providers
            .delete_bucket(provider_id, foundation_bucket_id)
            .await;
        return Err(error);
    }
    let response = response(&registry, region.id);
    if let Err(error) = scoped.commit().await {
        let _ = providers
            .delete_bucket(provider_id, foundation_bucket_id)
            .await;
        return Err(error);
    }
    Ok((StatusCode::CREATED, Json(response)))
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
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    lock_organization(tx, organization_id).await?;
    let mut registry = find_registry(tx, organization_id).await?;

    match active_gc_job(tx, &registry).await? {
        Some(job) if job.status == "running" => {
            return Err(AppError::Conflict(
                "Registry garbage collection is already running".into(),
            ));
        }
        Some(_) => {}
        None => {
            Operation::<RegistryGc>::new(tx, organization_id, "manual").await?;
        }
    }
    record_event(
        tx,
        organization_id,
        auth.actor_id,
        "registry-garbage-collection:queued",
        json!({ "summary": "Queued managed Registry garbage collection" }),
    )
    .await?;
    registry = find_registry(tx, organization_id).await?;
    let response = garbage_collection_response(tx, &registry, 1, 10).await?;
    scoped.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(response)))
}

async fn provision_metadata(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    foundation_bucket_id: Uuid,
) -> Result<managed_registry::Model, AppError> {
    let state = get_app_state();
    let credential_id = Uuid::new_v4();
    let secret_id = Uuid::new_v4();
    let access_key_id = format!("{ACCESS_KEY_PREFIX}{}", Uuid::new_v4().simple()).to_uppercase();
    let secret_access_key = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let plaintext = serde_json::to_vec(&S3SecretKey { secret_access_key })
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let ciphertext = lib::secrets::encrypt(
        &state.secrets,
        &buckets::tenant_key(organization_id),
        &plaintext,
    )
    .await?;
    secret::ActiveModel {
        id: Set(secret_id),
        scope: Set(SecretScope::Tenant),
        organization_id: Set(Some(organization_id)),
        ciphertext: Set(ciphertext),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    credential::ActiveModel {
        id: Set(credential_id),
        organization_id: Set(Some(organization_id)),
        access_key_id: Set(access_key_id),
        prefix: Set(String::new()),
        secret_id: Set(secret_id),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    bucket_grant::ActiveModel {
        id: Set(Uuid::new_v4()),
        credential_id: Set(credential_id),
        bucket_id: Set(foundation_bucket_id),
        organization_id: Set(Some(organization_id)),
        prefix: Set(String::new()),
        can_read: Set(true),
        can_write: Set(true),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    Ok(managed_registry::ActiveModel {
        organization_id: Set(organization_id),
        bucket_id: Set(foundation_bucket_id),
        credential_id: Set(credential_id),
        status: Set(ManagedRegistryStatus::Active),
        storage_revision: Set(Uuid::new_v4()),
        ..Default::default()
    }
    .insert(tx)
    .await?)
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
) -> Result<Json<ResolvedManagedRegistry>, AppError> {
    let state = get_app_state();
    let (registry, credential, credential_secret) =
        managed_registry::Entity::find_by_id(organization_id)
            .find_also_related(credential::Entity)
            .and_also_related(secret::Entity)
            .one(state.identity_db.connection())
            .await?
            .ok_or_else(|| AppError::NotFound("Managed Registry is not activated".into()))?;
    let credential = credential
        .filter(|credential| credential.organization_id == Some(organization_id))
        .filter(|credential| credential.revoked_at.is_none())
        .ok_or_else(|| AppError::NotFound("Managed Registry credential not found".into()))?;
    let credential_secret = credential_secret
        .filter(|secret| secret.scope == SecretScope::Tenant)
        .filter(|secret| secret.organization_id == Some(organization_id))
        .ok_or_else(|| AppError::NotFound("Managed Registry secret not found".into()))?;
    let grant = bucket_grant::Entity::find()
        .filter(bucket_grant::Column::CredentialId.eq(credential.id))
        .filter(bucket_grant::Column::OrganizationId.eq(organization_id))
        .filter(bucket_grant::Column::BucketId.eq(registry.bucket_id))
        .one(state.identity_db.connection())
        .await?
        .ok_or_else(|| AppError::NotFound("Managed Registry grant not found".into()))?;
    if !grant.can_read || !grant.can_write {
        return Err(AppError::Conflict(
            "Managed Registry grant must allow reads and writes".into(),
        ));
    }
    let plaintext = lib::secrets::decrypt(
        &state.secrets,
        &buckets::tenant_key(organization_id),
        &credential_secret.ciphertext,
    )
    .await?;
    let s3_secret: S3SecretKey = serde_json::from_slice(&plaintext)
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let organization = state
        .identity_db
        .connection()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT slug FROM organization WHERE id=$1 LIMIT 1",
            vec![organization_id.into()],
        ))
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".into()))?;
    Ok(Json(ResolvedManagedRegistry {
        organization_id,
        organization_slug: organization.try_get("", "slug")?,
        storage_revision: registry.storage_revision,
        status: status_name(&registry.status).into(),
        access_key_id: credential.access_key_id,
        secret_access_key: s3_secret.secret_access_key,
        bucket_name: REGISTRY_BUCKET_NAME.into(),
        storage_endpoint_url: state.config.storage_internal_url,
    }))
}

pub async fn require_active(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
) -> Result<managed_registry::Model, AppError> {
    let registry = managed_registry::Entity::find_by_id(organization_id)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::Conflict("Activate Managed Registry first".into()))?;
    if registry.status != ManagedRegistryStatus::Active {
        return Err(AppError::ServiceUnavailable(
            "Managed Registry is unavailable during maintenance".into(),
        ));
    }
    Ok(registry)
}

async fn find_registry(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
) -> Result<managed_registry::Model, AppError> {
    managed_registry::Entity::find_by_id(organization_id)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Managed Registry is not activated".into()))
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

async fn garbage_collection_response(
    tx: &DatabaseTransaction,
    registry: &managed_registry::Model,
    page: u64,
    per_page: u64,
) -> Result<RegistryGarbageCollectionResponse, AppError> {
    let active_job = active_gc_job(tx, registry)
        .await?
        .map(|job| RegistryGcJobResponse {
            id: job.id,
            status: job.status,
            trigger: job.trigger,
            available_at: job.available_at.to_rfc3339(),
        });
    let gc_runs = managed_registry_gc_run::Entity::find()
        .filter(managed_registry_gc_run::Column::OrganizationId.eq(registry.organization_id));
    let total = gc_runs.clone().count(tx).await?;
    let gc_runs = gc_runs
        .order_by_desc(managed_registry_gc_run::Column::StartedAt)
        .paginate(tx, per_page)
        .fetch_page(page - 1)
        .await?
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
    Ok(RegistryGarbageCollectionResponse {
        active_job,
        gc_runs: PaginatedResponse::new(gc_runs, total, page, per_page),
    })
}

async fn active_gc_job(
    tx: &DatabaseTransaction,
    registry: &managed_registry::Model,
) -> Result<Option<ActiveGcJob>, AppError> {
    let Some(job_id) = registry.gc_active_job_id else {
        return Ok(None);
    };
    let row = tx
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, status, COALESCE(payload->>'trigger', 'manual') AS trigger, available_at FROM worker_queue WHERE id=$1 AND organization_id=$2 AND queue_name=$3 AND job_type=$4 AND status IN ('queued', 'running') LIMIT 1",
            vec![
                job_id.into(),
                registry.organization_id.into(),
                Operation::<RegistryGc>::QUEUE.into(),
                Operation::<RegistryGc>::NAME.into(),
            ],
        ))
        .await?;
    row.map(|row| {
        Ok(ActiveGcJob {
            id: row.try_get("", "id")?,
            status: row.try_get("", "status")?,
            trigger: row.try_get("", "trigger")?,
            available_at: row.try_get("", "available_at")?,
        })
    })
    .transpose()
}

fn status_name(status: &ManagedRegistryStatus) -> &'static str {
    match status {
        ManagedRegistryStatus::Active => "active",
        ManagedRegistryStatus::Maintenance => "maintenance",
    }
}

async fn lock_organization(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
) -> Result<(), AppError> {
    tx.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM organization WHERE id=$1 FOR UPDATE",
        vec![organization_id.into()],
    ))
    .await?
    .ok_or_else(|| AppError::NotFound("Organization not found".into()))?;
    Ok(())
}
