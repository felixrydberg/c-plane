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
        bucket,
        region::{self, RegionRoutingMode, RegionStatus},
        storage_access_token_bucket,
    },
};

use super::databases::{verify_org_access, verify_project_in_org};

const BUCKET_PREFIX: &str = "cp-";

#[derive(Deserialize)]
pub struct ListBucketsQuery {
    pub project_id: Uuid,
}

#[derive(Serialize, ToSchema)]
pub struct BucketResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub region: Uuid,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateBucketRequest {
    pub project_id: Uuid,
    pub name: String,
    pub region: Uuid,
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/storage/buckets",
    request_body = CreateBucketRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
    ),
    responses(
        (status = 201, description = "Bucket created", body = BucketResponse),
        (status = 404, description = "Project or active region not found"),
        (status = 409, description = "Bucket name is unavailable"),
    ),
    tag = "storage",
)]
pub async fn create_bucket(
    AuthContext { tenant_db, auth }: AuthContext,
    axum::extract::Path(organization_id): axum::extract::Path<Uuid>,
    Json(body): Json<CreateBucketRequest>,
) -> Result<(axum::http::StatusCode, Json<BucketResponse>), AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let providers = crate::state::get_app_state()
        .s3_providers
        .ok_or_else(|| AppError::Internal("S3 provider service is not configured".into()))?;
    let name = body.name.trim().to_ascii_lowercase();
    if !valid_bucket_name(&name) {
        return Err(AppError::Project(crate::errors::ProjectError::InvalidSlug(
            "Invalid bucket name".into(),
        )));
    }

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, body.project_id, organization_id).await?;
    if bucket::Entity::find()
        .filter(bucket::Column::Name.eq(&name))
        .one(tx)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict("Bucket name is already reserved".into()));
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

    let bucket_id = Uuid::new_v4();
    let physical_bucket_name = physical_bucket_name(bucket_id);
    providers
        .create_bucket(provider_id, &physical_bucket_name)
        .await?;
    if let Err(error) = providers.ensure_bucket_sse_key(bucket_id).await {
        let _ = providers
            .delete_bucket(provider_id, &physical_bucket_name)
            .await;
        return Err(error);
    }

    let created = bucket::ActiveModel {
        id: Set(bucket_id),
        project_id: Set(body.project_id),
        organization_id: Set(organization_id),
        region: Set(body.region),
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
    scoped.commit().await?;

    Ok((axum::http::StatusCode::CREATED, Json(response(&created))))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/storage/buckets",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Query, description = "Project ID"),
    ),
    responses(
        (status = 200, description = "List of buckets", body = Vec<BucketResponse>),
        (status = 404, description = "Project not found"),
    ),
    tag = "storage",
)]
pub async fn list_buckets(
    AuthContext { tenant_db, .. }: AuthContext,
    axum::extract::Path(organization_id): axum::extract::Path<Uuid>,
    Query(query): Query<ListBucketsQuery>,
) -> Result<Json<Vec<BucketResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, query.project_id, organization_id).await?;

    let buckets = bucket::Entity::find()
        .filter(bucket::Column::ProjectId.eq(query.project_id))
        .order_by_asc(bucket::Column::Name)
        .all(tx)
        .await?;
    scoped.commit().await?;

    Ok(Json(buckets.iter().map(response).collect()))
}

#[utoipa::path(
    delete,
    path = "/api/organization/{organization_id}/storage/buckets/{bucket_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("bucket_id" = Uuid, Path, description = "Bucket ID"),
    ),
    responses(
        (status = 204, description = "Bucket deleted"),
        (status = 404, description = "Bucket or region not found"),
        (status = 409, description = "Provider bucket could not be deleted"),
    ),
    tag = "storage",
)]
pub async fn delete_bucket(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, bucket_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::http::StatusCode, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let providers = crate::state::get_app_state()
        .s3_providers
        .ok_or_else(|| AppError::Internal("S3 provider service is not configured".into()))?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let bucket = bucket::Entity::find_by_id(bucket_id)
        .filter(bucket::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Bucket not found".into()))?;
    verify_project_in_org(tx, bucket.project_id, organization_id).await?;
    let region = region::Entity::find_by_id(bucket.region)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Bucket region not found".into()))?;
    let provider_id = region
        .s3_provider_id
        .ok_or_else(|| AppError::Conflict("Region has no S3 provider".into()))?;
    let has_access_tokens = storage_access_token_bucket::Entity::find()
        .filter(storage_access_token_bucket::Column::BucketId.eq(bucket.id))
        .one(tx)
        .await?
        .is_some();

    providers
        .delete_bucket(provider_id, &physical_bucket_name(bucket.id))
        .await?;
    bucket::Entity::delete_by_id(bucket.id).exec(tx).await?;
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
    if has_access_tokens {
        providers.invalidate_access_token_cache().await?;
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

fn response(bucket: &bucket::Model) -> BucketResponse {
    BucketResponse {
        id: bucket.id,
        project_id: bucket.project_id,
        name: bucket.name.clone(),
        region: bucket.region,
    }
}

fn physical_bucket_name(id: Uuid) -> String {
    format!("{BUCKET_PREFIX}{}", id.simple())
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

    #[test]
    fn prefixes_physical_bucket_names() {
        assert_eq!(
            super::physical_bucket_name(uuid::Uuid::nil()),
            "cp-00000000000000000000000000000000"
        );
    }
}
