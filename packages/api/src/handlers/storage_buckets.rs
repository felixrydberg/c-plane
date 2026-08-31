use axum::{
    Json,
    extract::{Path, Query},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError,
    middleware::auth::AuthContext,
    models::entities::{
        bucket, bucket_grant, credential,
        region::{self, RegionRoutingMode, RegionStatus},
        storage,
    },
    services::buckets,
    state::get_app_state,
};

use super::databases::{verify_org_access, verify_org_owner, verify_project_in_org};

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
    verify_org_access(&tenant_db, organization_id)?;
    let name = body.name.trim().to_ascii_lowercase();
    if !valid_bucket_name(&name) {
        return Err(AppError::BadRequest("Invalid bucket name".into()));
    }

    let providers = get_app_state().s3_providers;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, body.project_id, organization_id).await?;
    if storage::Entity::find()
        .filter(storage::Column::ProjectId.eq(body.project_id))
        .filter(storage::Column::Name.eq(&name))
        .one(tx)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(
            "Bucket name is already reserved in this project".into(),
        ));
    }
    let region = region::Entity::find_by_id(body.region)
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
    let created = storage::ActiveModel {
        id: Set(Uuid::new_v4()),
        project_id: Set(body.project_id),
        organization_id: Set(organization_id),
        bucket_id: Set(foundation_bucket_id),
        name: Set(name.clone()),
    }
    .insert(tx)
    .await?;
    crate::services::events::record(
        tx,
        organization_id,
        body.project_id,
        "bucket:created",
        json!({ "summary": format!("Created bucket '{name}'"), "target_id": created.id }),
        auth.actor_id,
    )
    .await?;
    if let Err(error) = scoped.commit().await {
        let _ = providers
            .delete_bucket(provider_id, foundation_bucket_id)
            .await;
        return Err(error);
    }
    Ok((
        axum::http::StatusCode::CREATED,
        Json(response(&created, &region)),
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
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, query.project_id, organization_id).await?;
    let bucket_rows = storage::Entity::find()
        .filter(storage::Column::ProjectId.eq(query.project_id))
        .order_by_asc(storage::Column::Name)
        .find_also_related(bucket::Entity)
        .and_also_related(region::Entity)
        .all(tx)
        .await?;
    let buckets = bucket_rows
        .into_iter()
        .map(|(storage, bucket, region)| {
            bucket.ok_or_else(|| AppError::NotFound("Bucket foundation not found".into()))?;
            let region =
                region.ok_or_else(|| AppError::NotFound("Bucket region not found".into()))?;
            Ok(response(&storage, &region))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    scoped.commit().await?;
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
    verify_org_access(&tenant_db, organization_id)?;
    verify_org_owner(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let (bucket, foundation, region) = storage::Entity::find_by_id(bucket_id)
        .filter(storage::Column::OrganizationId.eq(organization_id))
        .find_also_related(bucket::Entity)
        .and_also_related(region::Entity)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Bucket not found".into()))?;
    verify_project_in_org(tx, bucket.project_id, organization_id).await?;
    foundation.ok_or_else(|| AppError::NotFound("Bucket foundation not found".into()))?;
    let region = region.ok_or_else(|| AppError::NotFound("Bucket region not found".into()))?;
    let provider_id = region
        .s3_provider_id
        .ok_or_else(|| AppError::Conflict("Region has no S3 provider".into()))?;
    let access_keys = bucket_grant::Entity::find()
        .filter(bucket_grant::Column::BucketId.eq(bucket.bucket_id))
        .find_also_related(credential::Entity)
        .all(tx)
        .await?
        .into_iter()
        .filter_map(|(_, credential)| credential.map(|credential| credential.access_key_id))
        .collect::<Vec<_>>();

    bucket::ActiveModel {
        id: Set(bucket.bucket_id),
        status: Set(bucket::BucketStatus::Deleting),
        ..Default::default()
    }
    .update(tx)
    .await?;
    scoped.commit().await?;
    get_app_state()
        .s3_providers
        .invalidate_access_token_caches(&access_keys)
        .await?;

    if !get_app_state()
        .s3_providers
        .bucket_is_empty(provider_id, bucket.bucket_id)
        .await?
    {
        let scoped = tenant_db.begin_scoped_transaction().await?;
        let tx = scoped.connection();
        bucket::ActiveModel {
            id: Set(bucket.bucket_id),
            status: Set(bucket::BucketStatus::Active),
            ..Default::default()
        }
        .update(tx)
        .await?;
        scoped.commit().await?;
        if let Err(error) = get_app_state()
            .s3_providers
            .invalidate_access_token_caches(&access_keys)
            .await
        {
            tracing::warn!(%error, %bucket_id, "bucket cache invalidation failed after deletion conflict");
        }
        return Err(AppError::Conflict(
            "Bucket must be empty before it can be deleted".into(),
        ));
    }

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    buckets::delete(tx, bucket.bucket_id).await?;
    crate::services::events::record(
        tx,
        organization_id,
        bucket.project_id,
        "bucket:deleted",
        json!({ "summary": format!("Deleted bucket '{}'", bucket.name), "target_id": bucket.id }),
        auth.actor_id,
    )
    .await?;
    scoped.commit().await?;
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

fn valid_bucket_name(name: &str) -> bool {
    (3..=63).contains(&name.len())
        && !name.starts_with(['.', '-'])
        && !name.ends_with(['.', '-'])
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::valid_bucket_name;
    #[test]
    fn validates_s3_bucket_name_basics() {
        assert!(valid_bucket_name("project-assets"));
        assert!(!valid_bucket_name("UPPERCASE"));
        assert!(!valid_bucket_name("ab"));
        assert!(!valid_bucket_name("-assets"));
    }
}
