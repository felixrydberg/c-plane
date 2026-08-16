use aws_sdk_s3::{
    config::{BehaviorVersion, Credentials, Region},
    types::{Delete, Object, ObjectIdentifier},
};
use axum::{
    Json,
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use subtle::ConstantTimeEq;
use tokio::{sync::Mutex, time::Instant};
use uuid::Uuid;

use crate::{auth::CredentialResolver, crypto::select_sse_key};

#[derive(Clone)]
pub struct InternalStorage {
    credentials: CredentialResolver,
    service_token: String,
    clients: Arc<Mutex<HashMap<Uuid, CachedClient>>>,
}

struct CachedClient {
    client: aws_sdk_s3::Client,
    expires_at: Instant,
}

const PROVIDER_CLIENT_TTL: Duration = Duration::from_secs(60);
const MAX_DELETE_PAGES: usize = 100;

#[derive(Clone, Deserialize)]
pub struct BucketDescriptor {
    pub organization_id: Uuid,
    pub bucket_id: Uuid,
    pub physical_bucket_name: String,
    pub provider_id: Uuid,
    pub platform_sse_key: String,
}

#[derive(Deserialize)]
pub struct ListObjectsRequest {
    #[serde(flatten)]
    pub bucket: BucketDescriptor,
    pub prefix: Option<String>,
    pub continuation_token: Option<String>,
    pub max_keys: i32,
}

#[derive(Deserialize)]
pub struct DownloadObjectRequest {
    #[serde(flatten)]
    pub bucket: BucketDescriptor,
    pub key: String,
}

#[derive(Deserialize)]
pub struct DeleteObjectsRequest {
    #[serde(flatten)]
    pub bucket: BucketDescriptor,
    pub key: Option<String>,
    pub prefix: Option<String>,
    pub continuation_token: Option<String>,
}

#[derive(Serialize)]
pub struct ListObjectsResponse {
    pub folders: Vec<String>,
    pub objects: Vec<StoredObject>,
    pub next_continuation_token: Option<String>,
}

#[derive(Serialize)]
pub struct StoredObject {
    pub key: String,
    pub size: i64,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
}

#[derive(Serialize)]
pub struct DeleteObjectsResponse {
    pub deleted: usize,
    pub next_continuation_token: Option<String>,
}

impl InternalStorage {
    pub fn new(credentials: CredentialResolver, service_token: String) -> Self {
        Self {
            credentials,
            service_token,
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get("x-cplane-token")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|provided| {
                provided
                    .as_bytes()
                    .ct_eq(self.service_token.as_bytes())
                    .into()
            })
    }

    async fn client(&self, bucket: &BucketDescriptor) -> Result<aws_sdk_s3::Client, StatusCode> {
        if let Some(cached) = self.clients.lock().await.get(&bucket.provider_id) {
            if cached.expires_at > Instant::now() {
                return Ok(cached.client.clone());
            }
        }
        let provider = self
            .credentials
            .provider(bucket.provider_id)
            .await
            .map_err(|error| {
                tracing::error!(%error, provider_id = %bucket.provider_id, "storage provider resolution failed");
                StatusCode::BAD_GATEWAY
            })?;
        let region = provider
            .provider_region
            .unwrap_or_else(|| "us-east-1".to_string());
        let client = aws_sdk_s3::Client::from_conf(
            aws_sdk_s3::Config::builder()
                .behavior_version(BehaviorVersion::latest())
                .endpoint_url(provider.endpoint_url)
                .region(Region::new(region))
                .credentials_provider(Credentials::new(
                    provider.access_key_id,
                    provider.secret_access_key,
                    provider.session_token,
                    None,
                    "c-plane-storage-console",
                ))
                .force_path_style(true)
                .build(),
        );
        self.clients.lock().await.insert(
            bucket.provider_id,
            CachedClient {
                client: client.clone(),
                expires_at: Instant::now() + PROVIDER_CLIENT_TTL,
            },
        );
        Ok(client)
    }
}

pub async fn list_objects(
    State(state): State<InternalStorage>,
    headers: HeaderMap,
    Json(request): Json<ListObjectsRequest>,
) -> Result<Json<ListObjectsResponse>, StatusCode> {
    if !state.authorized(&headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let client = state.client(&request.bucket).await?;
    let output = client
        .list_objects_v2()
        .bucket(&request.bucket.physical_bucket_name)
        .set_prefix(request.prefix)
        .set_continuation_token(request.continuation_token)
        .max_keys(request.max_keys.clamp(1, 100))
        .delimiter("/")
        .send()
        .await
        .map_err(|error| {
            let status = error
                .raw_response()
                .map(|response| response.status().as_u16());
            tracing::error!(%error, organization_id = %request.bucket.organization_id, bucket_id = %request.bucket.bucket_id, operation = "ListObjectsV2", "storage gateway operation failed");
            if status == Some(StatusCode::NOT_FOUND.as_u16()) {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_GATEWAY
            }
        })?;
    let folders = output
        .common_prefixes()
        .iter()
        .filter_map(|prefix| prefix.prefix().map(str::to_owned))
        .collect();
    let objects = output
        .contents()
        .iter()
        .filter(|object| !is_folder_marker(object))
        .filter_map(stored_object)
        .collect();
    tracing::info!(organization_id = %request.bucket.organization_id, bucket_id = %request.bucket.bucket_id, operation = "ListObjectsV2", outcome = "success", "storage gateway operation");
    Ok(Json(ListObjectsResponse {
        folders,
        objects,
        next_continuation_token: output.next_continuation_token().map(str::to_owned),
    }))
}

pub async fn download_object(
    State(state): State<InternalStorage>,
    headers: HeaderMap,
    Json(request): Json<DownloadObjectRequest>,
) -> Result<Response, StatusCode> {
    if !state.authorized(&headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let client = state.client(&request.bucket).await?;
    let sse = select_sse_key(None, None, None, &request.bucket.platform_sse_key)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let output = client
        .get_object()
        .bucket(&request.bucket.physical_bucket_name)
        .key(&request.key)
        .sse_customer_algorithm(sse.algorithm)
        .sse_customer_key(sse.key)
        .sse_customer_key_md5(sse.key_md5)
        .send()
        .await
        .map_err(|error| {
            let status = error
                .raw_response()
                .map(|response| response.status().as_u16());
            tracing::error!(%error, organization_id = %request.bucket.organization_id, bucket_id = %request.bucket.bucket_id, operation = "GetObject", "storage gateway operation failed");
            if status == Some(StatusCode::NOT_FOUND.as_u16()) {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_GATEWAY
            }
        })?;
    let content_type = output.content_type().map(str::to_owned);
    let content_length = output.content_length();
    let mut response = Response::new(Body::from_stream(tokio_util::io::ReaderStream::new(
        output.body.into_async_read(),
    )));
    if let Some(content_type) = content_type {
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_str(&content_type).map_err(|_| StatusCode::BAD_GATEWAY)?,
        );
    }
    if let Some(content_length) = content_length {
        response.headers_mut().insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&content_length.to_string()).expect("numeric header value"),
        );
    }
    tracing::info!(organization_id = %request.bucket.organization_id, bucket_id = %request.bucket.bucket_id, operation = "GetObject", outcome = "success", "storage gateway operation");
    Ok(response)
}

pub async fn delete_objects(
    State(state): State<InternalStorage>,
    headers: HeaderMap,
    Json(request): Json<DeleteObjectsRequest>,
) -> Result<Json<DeleteObjectsResponse>, StatusCode> {
    if !state.authorized(&headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let key = request.key.filter(|value| !value.is_empty());
    let prefix = request.prefix.filter(|value| !value.is_empty());
    if key.is_some() == prefix.is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let client = state.client(&request.bucket).await?;
    let result = if let Some(key) = key {
        client
            .delete_object()
            .bucket(&request.bucket.physical_bucket_name)
            .key(key)
            .send()
            .await
            .map_err(|error| {
                tracing::error!(%error, organization_id = %request.bucket.organization_id, bucket_id = %request.bucket.bucket_id, operation = "DeleteObject", "storage gateway operation failed");
                StatusCode::BAD_GATEWAY
            })?;
        DeletePrefixResult {
            deleted: 1,
            next_continuation_token: None,
        }
    } else if let Some(prefix) = prefix {
        delete_prefix(
            &client,
            &request.bucket.physical_bucket_name,
            &prefix,
            request.continuation_token,
        )
        .await?
    } else {
        DeletePrefixResult {
            deleted: 0,
            next_continuation_token: None,
        }
    };

    tracing::info!(organization_id = %request.bucket.organization_id, bucket_id = %request.bucket.bucket_id, deleted = result.deleted, partial = result.next_continuation_token.is_some(), operation = "DeleteObjects", outcome = "success", "storage gateway operation");
    Ok(Json(DeleteObjectsResponse {
        deleted: result.deleted,
        next_continuation_token: result.next_continuation_token,
    }))
}

struct DeletePrefixResult {
    deleted: usize,
    next_continuation_token: Option<String>,
}

async fn delete_prefix(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    prefix: &str,
    continuation_token: Option<String>,
) -> Result<DeletePrefixResult, StatusCode> {
    let prefix = folder_prefix(prefix);
    let mut continuation_token = continuation_token;
    let mut deleted = 0;
    let mut pages = 0;

    loop {
        let output = client
            .list_objects_v2()
            .bucket(bucket)
            .prefix(&prefix)
            .set_continuation_token(continuation_token)
            .send()
            .await
            .map_err(|error| {
                tracing::error!(%error, bucket, prefix, operation = "ListObjectsV2", "storage gateway operation failed while deleting prefix");
                StatusCode::BAD_GATEWAY
            })?;

        let identifiers = output
            .contents()
            .iter()
            .filter_map(|object| object.key())
            .map(|key| {
                ObjectIdentifier::builder()
                    .key(key)
                    .build()
                    .map_err(|error| {
                        tracing::error!(%error, bucket, prefix, operation = "DeleteObjects", "failed to build object delete request");
                        StatusCode::BAD_GATEWAY
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        if !identifiers.is_empty() {
            let batch_size = identifiers.len();
            let delete = Delete::builder()
                .set_objects(Some(identifiers))
                .quiet(true)
                .build()
                .map_err(|error| {
                    tracing::error!(%error, bucket, prefix, operation = "DeleteObjects", "failed to build object delete request");
                    StatusCode::BAD_GATEWAY
                })?;
            let response = client
                .delete_objects()
                .bucket(bucket)
                .delete(delete)
                .send()
                .await
                .map_err(|error| {
                    tracing::error!(%error, bucket, prefix, operation = "DeleteObjects", "storage gateway operation failed");
                    StatusCode::BAD_GATEWAY
                })?;
            if !response.errors().is_empty() {
                tracing::error!(bucket, prefix, errors = ?response.errors(), operation = "DeleteObjects", "storage provider returned delete errors");
                return Err(StatusCode::BAD_GATEWAY);
            }
            deleted += batch_size;
        }

        continuation_token = output.next_continuation_token().map(str::to_owned);
        pages += 1;
        if continuation_token.is_none() || pages == MAX_DELETE_PAGES {
            return Ok(DeletePrefixResult {
                deleted,
                next_continuation_token: continuation_token,
            });
        }
    }
}

fn folder_prefix(prefix: &str) -> String {
    if prefix.ends_with('/') {
        prefix.to_owned()
    } else {
        format!("{prefix}/")
    }
}

fn is_folder_marker(object: &Object) -> bool {
    object.key().is_some_and(|key| key.ends_with('/')) && object.size() == Some(0)
}

fn stored_object(object: &Object) -> Option<StoredObject> {
    Some(StoredObject {
        key: object.key()?.to_owned(),
        size: object.size().unwrap_or_default(),
        last_modified: object.last_modified().map(ToString::to_string),
        etag: object.e_tag().map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::InternalStorage;
    use crate::auth::CredentialResolver;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn requires_the_internal_service_token() {
        let storage = InternalStorage::new(
            CredentialResolver::new("http://api".into(), "unused".into()),
            "correct".into(),
        );
        let mut headers = HeaderMap::new();
        assert!(!storage.authorized(&headers));
        headers.insert("x-cplane-token", HeaderValue::from_static("wrong"));
        assert!(!storage.authorized(&headers));
        headers.insert("x-cplane-token", HeaderValue::from_static("correct"));
        assert!(storage.authorized(&headers));
    }

    #[test]
    fn normalizes_folder_prefixes() {
        assert_eq!(super::folder_prefix("folder"), "folder/");
        assert_eq!(super::folder_prefix("folder/"), "folder/");
    }
}
