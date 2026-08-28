use axum::{
    Json,
    body::Body,
    extract::{Path, Query},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError,
    middleware::auth::AuthContext,
    models::entities::{region, storage},
    state::{TenantDatabase, get_app_state},
};

use super::databases::verify_project_in_org;

const OBJECT_PAGE_SIZE: i32 = 100;

#[derive(Deserialize)]
pub struct ListObjectsQuery {
    pub prefix: Option<String>,
    pub continuation_token: Option<String>,
}

#[derive(Deserialize)]
pub struct DownloadObjectQuery {
    pub key: String,
}

#[derive(Deserialize)]
pub struct DeleteObjectsQuery {
    pub key: Option<String>,
    pub prefix: Option<String>,
    pub continuation_token: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct BucketObjectsResponse {
    pub folders: Vec<String>,
    pub objects: Vec<BucketObjectResponse>,
    pub next_continuation_token: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct BucketObjectResponse {
    pub key: String,
    pub size: i64,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
}

#[derive(Serialize)]
struct StorageBucketDescriptor {
    organization_id: Uuid,
    bucket_id: Uuid,
    physical_bucket_name: String,
    provider_id: Uuid,
    platform_sse_key: String,
}

#[derive(Serialize)]
struct StorageListObjectsRequest {
    #[serde(flatten)]
    bucket: StorageBucketDescriptor,
    prefix: Option<String>,
    continuation_token: Option<String>,
    max_keys: i32,
}

#[derive(Deserialize)]
struct StorageListObjectsResponse {
    folders: Vec<String>,
    objects: Vec<BucketObjectResponse>,
    next_continuation_token: Option<String>,
}

#[derive(Serialize)]
struct StorageDownloadObjectRequest {
    #[serde(flatten)]
    bucket: StorageBucketDescriptor,
    key: String,
}

#[derive(Serialize)]
struct StorageDeleteObjectsRequest {
    #[serde(flatten)]
    bucket: StorageBucketDescriptor,
    key: Option<String>,
    prefix: Option<String>,
    continuation_token: Option<String>,
}

#[derive(Deserialize)]
struct StorageDeleteObjectsResponse {
    deleted: usize,
    next_continuation_token: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct DeleteObjectsResponse {
    pub deleted: usize,
    pub next_continuation_token: Option<String>,
}

#[utoipa::path(
    get,
    operation_id = "storage_list_bucket_objects",
    path = "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("bucket_id" = Uuid, Path, description = "Bucket ID"),
        ("prefix" = Option<String>, Query, description = "Folder prefix"),
        ("continuation_token" = Option<String>, Query, description = "S3 continuation token"),
    ),
    responses(
        (status = 200, description = "Objects and folders in the bucket", body = BucketObjectsResponse),
        (status = 404, description = "Bucket not found"),
        (status = 503, description = "Storage gateway unavailable"),
    ),
    tag = "storage",
)]
pub async fn list_objects(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, bucket_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<ListObjectsQuery>,
) -> Result<Json<BucketObjectsResponse>, AppError> {
    let bucket = bucket_descriptor(&tenant_db, organization_id, bucket_id).await?;
    let response = storage_request(
        ".cplane/objects/list",
        StorageListObjectsRequest {
            bucket,
            prefix: query.prefix,
            continuation_token: query.continuation_token,
            max_keys: OBJECT_PAGE_SIZE,
        },
    )
    .await?;
    let payload = response
        .json::<StorageListObjectsResponse>()
        .await
        .map_err(|error| {
            AppError::ServiceUnavailable(format!("Invalid storage gateway response: {error}"))
        })?;
    Ok(Json(BucketObjectsResponse {
        folders: payload.folders,
        objects: payload.objects,
        next_continuation_token: payload.next_continuation_token,
    }))
}

#[utoipa::path(
    get,
    operation_id = "storage_download_bucket_object",
    path = "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects/download",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("bucket_id" = Uuid, Path, description = "Bucket ID"),
        ("key" = String, Query, description = "Exact object key"),
    ),
    responses(
        (status = 200, description = "Object download", body = [u8]),
        (status = 400, description = "Object key is required"),
        (status = 404, description = "Bucket or object not found"),
        (status = 503, description = "Storage gateway unavailable"),
    ),
    tag = "storage",
)]
pub async fn download_object(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, bucket_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<DownloadObjectQuery>,
) -> Result<Response, AppError> {
    if query.key.is_empty() {
        return Err(AppError::BadRequest("Object key is required".into()));
    }
    let bucket = bucket_descriptor(&tenant_db, organization_id, bucket_id).await?;
    let response = storage_request(
        ".cplane/objects/download",
        StorageDownloadObjectRequest {
            bucket,
            key: query.key.clone(),
        },
    )
    .await?;
    let content_type = response.headers().get(header::CONTENT_TYPE).cloned();
    let content_length = response.headers().get(header::CONTENT_LENGTH).cloned();
    let mut response = Response::new(Body::from_stream(response.bytes_stream()));
    if let Some(content_type) = content_type {
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, content_type);
    }
    if let Some(content_length) = content_length {
        response
            .headers_mut()
            .insert(header::CONTENT_LENGTH, content_length);
    }
    response
        .headers_mut()
        .insert(header::CONTENT_DISPOSITION, attachment_header(&query.key)?);
    Ok(response)
}

#[utoipa::path(
    delete,
    operation_id = "storage_delete_bucket_objects",
    path = "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("bucket_id" = Uuid, Path, description = "Bucket ID"),
        ("key" = Option<String>, Query, description = "Exact object key"),
        ("prefix" = Option<String>, Query, description = "Folder prefix"),
        ("continuation_token" = Option<String>, Query, description = "Continuation token for a partial folder deletion"),
    ),
    responses(
        (status = 204, description = "Object or folder deleted"),
        (status = 200, description = "Folder deletion partially completed", body = DeleteObjectsResponse),
        (status = 400, description = "Exactly one non-empty key or prefix is required"),
        (status = 404, description = "Bucket not found"),
        (status = 503, description = "Storage gateway unavailable"),
    ),
    tag = "storage",
)]
pub async fn delete_objects(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, bucket_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<DeleteObjectsQuery>,
) -> Result<Response, AppError> {
    let key = query.key.filter(|value| !value.is_empty());
    let prefix = query.prefix.filter(|value| !value.is_empty());
    if key.is_some() == prefix.is_some() {
        return Err(AppError::BadRequest(
            "Exactly one non-empty key or prefix is required".into(),
        ));
    }

    let bucket = bucket_descriptor(&tenant_db, organization_id, bucket_id).await?;
    let response = storage_request(
        ".cplane/objects/delete",
        StorageDeleteObjectsRequest {
            bucket,
            key,
            prefix,
            continuation_token: query.continuation_token,
        },
    )
    .await?
    .json::<StorageDeleteObjectsResponse>()
    .await
    .map_err(|error| {
        AppError::ServiceUnavailable(format!("Invalid storage gateway response: {error}"))
    })?;
    if let Some(next_continuation_token) = response.next_continuation_token {
        return Ok((
            StatusCode::OK,
            Json(DeleteObjectsResponse {
                deleted: response.deleted,
                next_continuation_token: Some(next_continuation_token),
            }),
        )
            .into_response());
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn bucket_descriptor(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    bucket_id: Uuid,
) -> Result<StorageBucketDescriptor, AppError> {
    super::databases::verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let bucket = storage::Entity::find_by_id(bucket_id)
        .filter(storage::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Bucket not found".into()))?;
    verify_project_in_org(tx, bucket.project_id, organization_id).await?;
    let region = region::Entity::find_by_id(bucket.region_id)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Bucket region not found".into()))?;
    let provider_id = region
        .s3_provider_id
        .ok_or_else(|| AppError::Conflict("Region has no S3 provider".into()))?;
    scoped.commit().await?;
    let platform_sse_key = get_app_state().s3_providers.bucket_key(bucket_id).await?;
    Ok(StorageBucketDescriptor {
        organization_id,
        bucket_id,
        physical_bucket_name: lib::buckets::physical_bucket_name(bucket_id),
        provider_id,
        platform_sse_key,
    })
}

async fn storage_request<T: Serialize>(path: &str, body: T) -> Result<reqwest::Response, AppError> {
    let state = get_app_state();
    let response = state
        .storage_client
        .post(format!(
            "{}/{}",
            state.config.storage_internal_url.trim_end_matches('/'),
            path
        ))
        .header("x-cplane-token", &state.config.service_token)
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            AppError::ServiceUnavailable(format!("Storage gateway request failed: {error}"))
        })?;
    if response.status().is_success() {
        Ok(response)
    } else if response.status() == StatusCode::NOT_FOUND {
        Err(AppError::NotFound("Object not found".into()))
    } else {
        Err(AppError::ServiceUnavailable(format!(
            "Storage gateway returned {}",
            response.status()
        )))
    }
}

fn attachment_header(key: &str) -> Result<HeaderValue, AppError> {
    let filename = key
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or("download")
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => character,
            _ => '_',
        })
        .collect::<String>();
    HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
        .map_err(|_| AppError::Internal("Invalid download filename".into()))
}

#[cfg(test)]
mod tests {
    use super::attachment_header;

    #[test]
    fn uses_a_safe_attachment_filename() {
        assert_eq!(
            attachment_header("reports/quarter one.csv").unwrap(),
            "attachment; filename=\"quarter_one.csv\""
        );
    }
}
