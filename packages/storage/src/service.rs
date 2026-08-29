use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use axum::{
    Router,
    error_handling::HandleError,
    http::{Response, StatusCode},
    routing::get,
};
use s3s::{
    Body, HttpError, S3, S3Request, S3Response, S3Result, dto::*, service::S3ServiceBuilder,
};
use s3s_aws::Proxy;

use crate::{
    auth::{BucketPermission, CredentialIdentity, CredentialResolver},
    config::Config,
    crypto::select_sse_key,
    internal::{self, InternalStorage},
};

pub struct StorageService {
    config: Config,
    credentials: CredentialResolver,
}

#[derive(Clone)]
struct ProviderProxy {
    credentials: CredentialResolver,
}

#[async_trait::async_trait]
impl S3 for ProviderProxy {
    async fn list_buckets(
        &self,
        request: S3Request<ListBucketsInput>,
    ) -> S3Result<S3Response<ListBucketsOutput>> {
        let identity = identity(&request)?;
        let mut names = accessible_bucket_names(identity);
        names.sort();
        let prefix = request.input.prefix.as_deref().unwrap_or_default();
        let start_after = request
            .input
            .continuation_token
            .as_deref()
            .unwrap_or_default();
        let limit = request.input.max_buckets.unwrap_or(10_000).max(0) as usize;
        let mut names = names
            .into_iter()
            .filter(|name| name.starts_with(prefix) && name.as_str() > start_after)
            .collect::<Vec<_>>();
        let continuation_token =
            (names.len() > limit && limit > 0).then(|| names[limit - 1].clone());
        names.truncate(limit);
        Ok(S3Response::new(ListBucketsOutput {
            buckets: Some(
                names
                    .into_iter()
                    .map(|name| Bucket {
                        bucket_region: identity
                            .bucket_permissions
                            .iter()
                            .find(|permission| permission.bucket_name == name)
                            .map(|permission| permission.region.clone()),
                        name: Some(name),
                        ..Default::default()
                    })
                    .collect(),
            ),
            continuation_token,
            prefix: request.input.prefix,
            ..Default::default()
        }))
    }

    async fn head_bucket(
        &self,
        mut request: S3Request<HeadBucketInput>,
    ) -> S3Result<S3Response<HeadBucketOutput>> {
        let target = self.target(&request, &request.input.bucket)?;
        request.input.bucket = target.physical_bucket_name.clone();
        let mut response = self
            .provider(&target)
            .await?
            .head_bucket(request)
            .await
            .map_err(sanitize_provider_error)?;
        response.output.bucket_region = Some(target.region);
        response.output.access_point_alias = None;
        response.output.bucket_location_name = None;
        response.output.bucket_location_type = None;
        Ok(response)
    }

    async fn get_bucket_location(
        &self,
        request: S3Request<GetBucketLocationInput>,
    ) -> S3Result<S3Response<GetBucketLocationOutput>> {
        let target = self.target(&request, &request.input.bucket)?;
        Ok(S3Response::new(GetBucketLocationOutput {
            location_constraint: bucket_location_constraint(&target.region),
        }))
    }

    async fn delete_object(
        &self,
        mut request: S3Request<DeleteObjectInput>,
    ) -> S3Result<S3Response<DeleteObjectOutput>> {
        let target = self.target(&request, &request.input.bucket)?;
        ensure_key_allowed(&identity(&request)?.prefix, &request.input.key)?;
        request.input.bucket = target.physical_bucket_name.clone();
        self.provider(&target)
            .await?
            .delete_object(request)
            .await
            .map_err(sanitize_provider_error)
    }

    async fn delete_objects(
        &self,
        mut request: S3Request<DeleteObjectsInput>,
    ) -> S3Result<S3Response<DeleteObjectsOutput>> {
        let target = self.target(&request, &request.input.bucket)?;
        for object in &request.input.delete.objects {
            ensure_key_allowed(&identity(&request)?.prefix, &object.key)?;
        }
        request.input.bucket = target.physical_bucket_name.clone();
        self.provider(&target)
            .await?
            .delete_objects(request)
            .await
            .map_err(sanitize_provider_error)
    }

    async fn list_objects(
        &self,
        mut request: S3Request<ListObjectsInput>,
    ) -> S3Result<S3Response<ListObjectsOutput>> {
        let logical_bucket = request.input.bucket.clone();
        let target = self.target(&request, &request.input.bucket)?;
        request.input.prefix = Some(scoped_list_prefix(
            &identity(&request)?.prefix,
            request.input.prefix.as_deref(),
        )?);
        request.input.bucket = target.physical_bucket_name.clone();
        let mut response = self
            .provider(&target)
            .await?
            .list_objects(request)
            .await
            .map_err(sanitize_provider_error)?;
        response.output.name = Some(logical_bucket);
        Ok(response)
    }

    async fn list_objects_v2(
        &self,
        mut request: S3Request<ListObjectsV2Input>,
    ) -> S3Result<S3Response<ListObjectsV2Output>> {
        let logical_bucket = request.input.bucket.clone();
        let target = self.target(&request, &request.input.bucket)?;
        request.input.prefix = Some(scoped_list_prefix(
            &identity(&request)?.prefix,
            request.input.prefix.as_deref(),
        )?);
        request.input.bucket = target.physical_bucket_name.clone();
        let mut response = self
            .provider(&target)
            .await?
            .list_objects_v2(request)
            .await
            .map_err(sanitize_provider_error)?;
        response.output.name = Some(logical_bucket);
        Ok(response)
    }

    async fn abort_multipart_upload(
        &self,
        mut request: S3Request<AbortMultipartUploadInput>,
    ) -> S3Result<S3Response<AbortMultipartUploadOutput>> {
        let target = self.target(&request, &request.input.bucket)?;
        ensure_key_allowed(&identity(&request)?.prefix, &request.input.key)?;
        request.input.bucket = target.physical_bucket_name.clone();
        self.provider(&target)
            .await?
            .abort_multipart_upload(request)
            .await
            .map_err(sanitize_provider_error)
    }

    async fn list_multipart_uploads(
        &self,
        mut request: S3Request<ListMultipartUploadsInput>,
    ) -> S3Result<S3Response<ListMultipartUploadsOutput>> {
        let logical_bucket = request.input.bucket.clone();
        let target = self.target(&request, &request.input.bucket)?;
        request.input.prefix = Some(scoped_list_prefix(
            &identity(&request)?.prefix,
            request.input.prefix.as_deref(),
        )?);
        request.input.bucket = target.physical_bucket_name.clone();
        let mut response = self
            .provider(&target)
            .await?
            .list_multipart_uploads(request)
            .await
            .map_err(sanitize_provider_error)?;
        response.output.bucket = Some(logical_bucket);
        Ok(response)
    }

    async fn list_parts(
        &self,
        mut request: S3Request<ListPartsInput>,
    ) -> S3Result<S3Response<ListPartsOutput>> {
        let logical_bucket = request.input.bucket.clone();
        let target = self.target(&request, &request.input.bucket)?;
        ensure_key_allowed(&identity(&request)?.prefix, &request.input.key)?;
        request.input.bucket = target.physical_bucket_name.clone();
        let mut response = self
            .provider(&target)
            .await?
            .list_parts(request)
            .await
            .map_err(sanitize_provider_error)?;
        response.output.bucket = Some(logical_bucket);
        Ok(response)
    }

    async fn get_object(
        &self,
        mut request: S3Request<GetObjectInput>,
    ) -> S3Result<S3Response<GetObjectOutput>> {
        let target = self.target(&request, &request.input.bucket)?;
        ensure_key_allowed(&identity(&request)?.prefix, &request.input.key)?;
        fill_sse(
            &mut request.input.sse_customer_algorithm,
            &mut request.input.sse_customer_key,
            &mut request.input.sse_customer_key_md5,
            &target.platform_sse_key,
        )?;
        request.input.bucket = target.physical_bucket_name.clone();
        self.provider(&target)
            .await?
            .get_object(request)
            .await
            .map_err(sanitize_provider_error)
    }

    async fn head_object(
        &self,
        mut request: S3Request<HeadObjectInput>,
    ) -> S3Result<S3Response<HeadObjectOutput>> {
        let target = self.target(&request, &request.input.bucket)?;
        ensure_key_allowed(&identity(&request)?.prefix, &request.input.key)?;
        fill_sse(
            &mut request.input.sse_customer_algorithm,
            &mut request.input.sse_customer_key,
            &mut request.input.sse_customer_key_md5,
            &target.platform_sse_key,
        )?;
        request.input.bucket = target.physical_bucket_name.clone();
        self.provider(&target)
            .await?
            .head_object(request)
            .await
            .map_err(sanitize_provider_error)
    }

    async fn put_object(
        &self,
        mut request: S3Request<PutObjectInput>,
    ) -> S3Result<S3Response<PutObjectOutput>> {
        let target = self.target(&request, &request.input.bucket)?;
        ensure_key_allowed(&identity(&request)?.prefix, &request.input.key)?;
        fill_sse(
            &mut request.input.sse_customer_algorithm,
            &mut request.input.sse_customer_key,
            &mut request.input.sse_customer_key_md5,
            &target.platform_sse_key,
        )?;
        request.input.bucket = target.physical_bucket_name.clone();
        self.provider(&target)
            .await?
            .put_object(request)
            .await
            .map_err(sanitize_provider_error)
    }

    async fn create_multipart_upload(
        &self,
        mut request: S3Request<CreateMultipartUploadInput>,
    ) -> S3Result<S3Response<CreateMultipartUploadOutput>> {
        let logical_bucket = request.input.bucket.clone();
        let target = self.target(&request, &request.input.bucket)?;
        ensure_key_allowed(&identity(&request)?.prefix, &request.input.key)?;
        fill_sse(
            &mut request.input.sse_customer_algorithm,
            &mut request.input.sse_customer_key,
            &mut request.input.sse_customer_key_md5,
            &target.platform_sse_key,
        )?;
        request.input.bucket = target.physical_bucket_name.clone();
        let mut response = self
            .provider(&target)
            .await?
            .create_multipart_upload(request)
            .await
            .map_err(sanitize_provider_error)?;
        response.output.bucket = Some(logical_bucket);
        Ok(response)
    }

    async fn upload_part(
        &self,
        mut request: S3Request<UploadPartInput>,
    ) -> S3Result<S3Response<UploadPartOutput>> {
        let target = self.target(&request, &request.input.bucket)?;
        ensure_key_allowed(&identity(&request)?.prefix, &request.input.key)?;
        fill_sse(
            &mut request.input.sse_customer_algorithm,
            &mut request.input.sse_customer_key,
            &mut request.input.sse_customer_key_md5,
            &target.platform_sse_key,
        )?;
        request.input.bucket = target.physical_bucket_name.clone();
        self.provider(&target)
            .await?
            .upload_part(request)
            .await
            .map_err(sanitize_provider_error)
    }

    async fn complete_multipart_upload(
        &self,
        mut request: S3Request<CompleteMultipartUploadInput>,
    ) -> S3Result<S3Response<CompleteMultipartUploadOutput>> {
        let logical_bucket = request.input.bucket.clone();
        let target = self.target(&request, &request.input.bucket)?;
        ensure_key_allowed(&identity(&request)?.prefix, &request.input.key)?;
        fill_sse(
            &mut request.input.sse_customer_algorithm,
            &mut request.input.sse_customer_key,
            &mut request.input.sse_customer_key_md5,
            &target.platform_sse_key,
        )?;
        request.input.bucket = target.physical_bucket_name.clone();
        let mut response = self
            .provider(&target)
            .await?
            .complete_multipart_upload(request)
            .await
            .map_err(sanitize_provider_error)?;
        response.output.bucket = Some(logical_bucket);
        response.output.location = None;
        Ok(response)
    }

    async fn copy_object(
        &self,
        mut request: S3Request<CopyObjectInput>,
    ) -> S3Result<S3Response<CopyObjectOutput>> {
        let destination = self.target(&request, &request.input.bucket)?;
        ensure_key_allowed(&identity(&request)?.prefix, &request.input.key)?;
        let source = self.copy_source_target(&request, &request.input.copy_source)?;
        rewrite_copy_source(&mut request.input.copy_source, &source.physical_bucket_name)?;
        same_provider(&source, &destination)?;
        fill_sse(
            &mut request.input.sse_customer_algorithm,
            &mut request.input.sse_customer_key,
            &mut request.input.sse_customer_key_md5,
            &destination.platform_sse_key,
        )?;
        fill_sse(
            &mut request.input.copy_source_sse_customer_algorithm,
            &mut request.input.copy_source_sse_customer_key,
            &mut request.input.copy_source_sse_customer_key_md5,
            &source.platform_sse_key,
        )?;
        request.input.bucket = destination.physical_bucket_name.clone();
        self.provider(&destination)
            .await?
            .copy_object(request)
            .await
            .map_err(sanitize_provider_error)
    }

    async fn upload_part_copy(
        &self,
        mut request: S3Request<UploadPartCopyInput>,
    ) -> S3Result<S3Response<UploadPartCopyOutput>> {
        let destination = self.target(&request, &request.input.bucket)?;
        ensure_key_allowed(&identity(&request)?.prefix, &request.input.key)?;
        let source = self.copy_source_target(&request, &request.input.copy_source)?;
        rewrite_copy_source(&mut request.input.copy_source, &source.physical_bucket_name)?;
        same_provider(&source, &destination)?;
        fill_sse(
            &mut request.input.sse_customer_algorithm,
            &mut request.input.sse_customer_key,
            &mut request.input.sse_customer_key_md5,
            &destination.platform_sse_key,
        )?;
        fill_sse(
            &mut request.input.copy_source_sse_customer_algorithm,
            &mut request.input.copy_source_sse_customer_key,
            &mut request.input.copy_source_sse_customer_key_md5,
            &source.platform_sse_key,
        )?;
        request.input.bucket = destination.physical_bucket_name.clone();
        self.provider(&destination)
            .await?
            .upload_part_copy(request)
            .await
            .map_err(sanitize_provider_error)
    }
}

impl ProviderProxy {
    fn target<T>(&self, request: &S3Request<T>, bucket: &str) -> S3Result<BucketPermission> {
        identity(request)?
            .bucket_permissions
            .iter()
            .find(|permission| permission.bucket_name == bucket)
            .cloned()
            .ok_or_else(|| s3s::s3_error!(AccessDenied))
    }

    fn copy_source_target<T>(
        &self,
        request: &S3Request<T>,
        copy_source: &CopySource,
    ) -> S3Result<BucketPermission> {
        let source = copy_source.format_to_string();
        let (bucket, _) = source
            .split_once('/')
            .ok_or_else(|| s3s::s3_error!(InvalidRequest, "Invalid copy source"))?;
        let target = self.target(request, bucket)?;
        if !target.can_read {
            return Err(s3s::s3_error!(AccessDenied));
        }
        ensure_key_allowed(&identity(request)?.prefix, copy_source_key(copy_source))?;
        Ok(target)
    }

    async fn provider(&self, target: &BucketPermission) -> S3Result<Proxy> {
        let provider = self.credentials.provider(target.provider_id).await.map_err(|error| {
            tracing::error!(%error, provider_id = %target.provider_id, "provider resolver failed");
            s3s::s3_error!(InternalError)
        })?;
        let region = provider
            .provider_region
            .clone()
            .ok_or_else(|| s3s::s3_error!(InternalError, "Provider region is not configured"))?;
        // build per request; cache clients by provider ID if connection setup becomes measurable.
        let config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .endpoint_url(provider.endpoint_url.clone())
            .region(Region::new(region))
            .credentials_provider(Credentials::new(
                provider.access_key_id.clone(),
                provider.secret_access_key.clone(),
                provider.session_token.clone(),
                None,
                "c-plane-control-plane",
            ))
            .force_path_style(true)
            .build();
        Ok(Proxy::from(aws_sdk_s3::Client::from_conf(config)))
    }
}

fn identity<T>(request: &S3Request<T>) -> S3Result<&CredentialIdentity> {
    request
        .extensions
        .get::<CredentialIdentity>()
        .ok_or_else(|| s3s::s3_error!(AccessDenied))
}

fn ensure_key_allowed(prefix: &str, key: &str) -> S3Result<()> {
    if key.starts_with(prefix) {
        Ok(())
    } else {
        Err(s3s::s3_error!(AccessDenied))
    }
}

fn scoped_list_prefix(allowed: &str, requested: Option<&str>) -> S3Result<String> {
    let requested = requested.unwrap_or_default();
    if requested.is_empty() {
        Ok(allowed.to_owned())
    } else if requested.starts_with(allowed) {
        Ok(requested.to_owned())
    } else {
        Err(s3s::s3_error!(AccessDenied))
    }
}

fn copy_source_key(copy_source: &CopySource) -> &str {
    match copy_source {
        CopySource::Bucket { key, .. }
        | CopySource::AccessPoint { key, .. }
        | CopySource::Outpost { key, .. } => key,
    }
}

fn accessible_bucket_names(identity: &CredentialIdentity) -> Vec<String> {
    let mut names = identity
        .bucket_permissions
        .iter()
        .filter(|permission| permission.can_read || permission.can_write)
        .map(|permission| permission.bucket_name.clone())
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn bucket_location_constraint(region: &str) -> Option<BucketLocationConstraint> {
    (region != "us-east-1").then(|| region.to_owned().into())
}

fn sanitize_provider_error(error: s3s::S3Error) -> s3s::S3Error {
    let mut sanitized = s3s::S3Error::new(error.code().clone());
    sanitized.set_message("Storage provider request failed");
    if let Some(request_id) = error.request_id() {
        sanitized.set_request_id(request_id);
    }
    if let Some(status_code) = error.status_code() {
        sanitized.set_status_code(status_code);
    }
    sanitized
}

fn fill_sse(
    algorithm: &mut Option<String>,
    key: &mut Option<String>,
    key_md5: &mut Option<String>,
    platform_key: &str,
) -> S3Result<()> {
    let selected = select_sse_key(
        algorithm.as_deref(),
        key.as_deref(),
        key_md5.as_deref(),
        platform_key,
    )
    .map_err(|error| s3s::s3_error!(InvalidRequest, "{error}"))?;
    *algorithm = Some(selected.algorithm);
    *key = Some(selected.key);
    *key_md5 = Some(selected.key_md5);
    Ok(())
}

fn rewrite_copy_source(copy_source: &mut CopySource, physical_bucket: &str) -> S3Result<()> {
    let source = copy_source.format_to_string();
    let (_, key) = source
        .split_once('/')
        .ok_or_else(|| s3s::s3_error!(InvalidRequest, "Invalid copy source"))?;
    *copy_source = CopySource::parse(&format!("{physical_bucket}/{key}"))
        .map_err(|_| s3s::s3_error!(InvalidRequest, "Invalid copy source"))?;
    Ok(())
}

fn same_provider(source: &BucketPermission, destination: &BucketPermission) -> S3Result<()> {
    if source.provider_id == destination.provider_id {
        Ok(())
    } else {
        Err(s3s::s3_error!(
            InvalidRequest,
            "Cross-provider copies are not supported"
        ))
    }
}

impl StorageService {
    pub async fn from_config(config: Config) -> Result<Self, reqwest::Error> {
        let credentials =
            CredentialResolver::new(config.api_url.clone(), config.internal_token.clone());
        Ok(Self {
            config,
            credentials,
        })
    }

    pub async fn serve(self) -> Result<(), std::io::Error> {
        let credentials = self.credentials.clone();
        let mut builder = S3ServiceBuilder::new(ProviderProxy {
            credentials: credentials.clone(),
        });
        builder.set_auth(credentials.clone());
        builder.set_access(credentials.clone());
        let s3 = HandleError::new(builder.build(), handle_s3_error);
        let app = Router::new()
            .route("/health", get(|| async { "OK" }))
            .route(
                "/.cplane/objects/list",
                axum::routing::post(internal::list_objects),
            )
            .route(
                "/.cplane/objects/download",
                axum::routing::post(internal::download_object),
            )
            .route(
                "/.cplane/objects/delete",
                axum::routing::post(internal::delete_objects),
            )
            .with_state(InternalStorage::new(
                credentials,
                self.config.internal_token.clone(),
            ))
            .fallback_service(s3);
        let listener = tokio::net::TcpListener::bind(self.config.listen).await?;
        tracing::info!(address = %self.config.listen, "storage endpoint started");
        axum::serve(listener, app).await
    }
}

async fn handle_s3_error(_error: HttpError) -> Response<Body> {
    tracing::error!("S3 HTTP request failed");
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from("Internal Server Error".to_string()))
        .expect("static response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_copy_source_bucket_without_touching_the_key() {
        let mut rewritten = CopySource::parse("uploads/path/to/file.txt?versionId=1").unwrap();
        rewrite_copy_source(&mut rewritten, "cp-123").unwrap();
        assert_eq!(
            rewritten.format_to_string(),
            "cp-123/path/to/file.txt?versionId=1"
        );
    }

    #[test]
    fn lists_readable_and_writable_buckets_across_regions() {
        let identity = CredentialIdentity {
            organization_id: Some(uuid::Uuid::nil()),
            project_id: Some(uuid::Uuid::nil()),
            credential_id: uuid::Uuid::nil(),
            prefix: "backups/production/".into(),
            bucket_permissions: vec![
                BucketPermission {
                    bucket_id: uuid::Uuid::nil(),
                    bucket_name: "write-only".into(),
                    physical_bucket_name: "cp-write".into(),
                    region: "eu-north-1".into(),
                    provider_id: uuid::Uuid::nil(),
                    platform_sse_key: "key".into(),
                    can_read: false,
                    can_write: true,
                },
                BucketPermission {
                    bucket_id: uuid::Uuid::nil(),
                    bucket_name: "read-only".into(),
                    physical_bucket_name: "cp-read".into(),
                    region: "us-east-1".into(),
                    provider_id: uuid::Uuid::nil(),
                    platform_sse_key: "key".into(),
                    can_read: true,
                    can_write: false,
                },
            ],
        };
        assert_eq!(
            accessible_bucket_names(&identity),
            ["read-only", "write-only"]
        );
    }

    #[test]
    fn resolves_bucket_location_constraint_from_c_plane_region() {
        assert_eq!(
            bucket_location_constraint("eu-north-1").map(|location| location.as_str().to_owned()),
            Some("eu-north-1".into())
        );
        assert_eq!(bucket_location_constraint("us-east-1"), None);
    }

    #[test]
    fn sanitizes_provider_error_details() {
        let mut error = s3s::S3Error::with_message(
            s3s::S3ErrorCode::AccessDenied,
            "physical bucket cp-secret leaked",
        );
        error.set_request_id("request-id");

        let sanitized = sanitize_provider_error(error);

        assert_eq!(sanitized.code(), &s3s::S3ErrorCode::AccessDenied);
        assert_eq!(sanitized.message(), Some("Storage provider request failed"));
        assert_eq!(sanitized.request_id(), Some("request-id"));
        assert!(!format!("{sanitized:?}").contains("cp-secret"));
    }

    #[test]
    fn enforces_credential_prefixes_for_keys_and_listings() {
        assert_eq!(
            scoped_list_prefix("backups/production/", None).unwrap(),
            "backups/production/"
        );
        assert_eq!(
            scoped_list_prefix("backups/production/", Some("backups/production/daily/")).unwrap(),
            "backups/production/daily/"
        );
        assert!(scoped_list_prefix("backups/production/", Some("backups/staging/")).is_err());
        assert!(ensure_key_allowed("backups/production/", "backups/production/file").is_ok());
        assert!(ensure_key_allowed("backups/production/", "backups/staging/file").is_err());
    }
}
