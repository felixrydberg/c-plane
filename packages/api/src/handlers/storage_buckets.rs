use axum::{
    Json,
    extract::{Path, Query},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError,
    middleware::auth::AuthContext,
    models::entities::{region, storage},
    state::get_app_state,
};
use lib::services::storage_buckets::{self, CreateStorageBucketInput};

#[derive(Deserialize)]
pub struct ListBucketsQuery {
    pub project_id: Uuid,
}

#[derive(Serialize, ToSchema)]
pub struct BucketRegionResponse {
    pub label: String,
    pub slug: String,
}

#[derive(Serialize, ToSchema)]
pub struct BucketResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub region: BucketRegionResponse,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateBucketRequest {
    pub project_id: Uuid,
    pub name: String,
    pub region: Uuid,
}

#[utoipa::path(
    post, path = "/api/organization/{organization_id}/storage/buckets", request_body = CreateBucketRequest,
    params(("organization_id" = Uuid, Path, description = "Organization ID")),
    responses((status = 201, description = "Bucket created", body = BucketResponse), (status = 404, description = "Project or active region not found"), (status = 409, description = "Bucket name is unavailable")), tag = "storage",
)]
pub async fn create_bucket(
    AuthContext { tenant_db, auth }: AuthContext,
    Path(organization_id): Path<Uuid>,
    Json(body): Json<CreateBucketRequest>,
) -> Result<(axum::http::StatusCode, Json<BucketResponse>), AppError> {
    let state = get_app_state();
    let created = storage_buckets::create(
        &tenant_db,
        &state.s3_providers,
        &state.secrets,
        organization_id,
        auth.actor_id,
        CreateStorageBucketInput {
            project_id: body.project_id,
            name: body.name,
            region_id: body.region,
        },
    )
    .await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(response(&created.bucket, &created.region)),
    ))
}

#[utoipa::path(
    get, path = "/api/organization/{organization_id}/storage/buckets",
    params(("organization_id" = Uuid, Path, description = "Organization ID"), ("project_id" = Uuid, Query, description = "Project ID")),
    responses((status = 200, description = "List of buckets", body = Vec<BucketResponse>), (status = 404, description = "Project not found")), tag = "storage",
)]
pub async fn list_buckets(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
    Query(query): Query<ListBucketsQuery>,
) -> Result<Json<Vec<BucketResponse>>, AppError> {
    let buckets = storage_buckets::list(&tenant_db, organization_id, query.project_id)
        .await?
        .into_iter()
        .map(|item| response(&item.bucket, &item.region))
        .collect();
    Ok(Json(buckets))
}

#[utoipa::path(
    delete, path = "/api/organization/{organization_id}/storage/buckets/{bucket_id}",
    params(("organization_id" = Uuid, Path, description = "Organization ID"), ("bucket_id" = Uuid, Path, description = "Bucket ID")),
    responses((status = 204, description = "Bucket deleted"), (status = 404, description = "Bucket or region not found"), (status = 409, description = "Provider bucket could not be deleted")), tag = "storage",
)]
pub async fn delete_bucket(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, bucket_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::http::StatusCode, AppError> {
    let deleted = storage_buckets::delete(
        &tenant_db,
        &get_app_state().s3_providers,
        organization_id,
        bucket_id,
        auth.actor_id,
    )
    .await?;
    let _ = deleted;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

fn response(bucket: &storage::Model, region: &region::Model) -> BucketResponse {
    BucketResponse {
        id: bucket.id,
        project_id: bucket.project_id,
        name: bucket.name.clone(),
        region: BucketRegionResponse {
            label: region.display_name.clone(),
            slug: region.slug.clone(),
        },
    }
}
