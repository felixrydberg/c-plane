use async_trait::async_trait;
use reqwest::StatusCode;
use s3s::{
    S3Result,
    access::{S3Access, S3AccessContext},
    auth::{S3Auth, SecretKey},
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone, Deserialize)]
pub struct S3Provider {
    pub endpoint_url: String,
    pub provider_region: Option<String>,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

#[derive(Clone)]
pub struct CredentialResolver {
    http: reqwest::Client,
    api_url: String,
    service_token: String,
}

#[derive(Clone, Deserialize)]
pub struct ResolvedCredential {
    pub organization_id: Option<Uuid>,
    pub credential_id: Uuid,
    #[serde(default)]
    pub prefix: String,
    pub bucket_permissions: Vec<BucketPermission>,
    pub secret_access_key: String,
}

#[derive(Clone, Deserialize)]
pub struct BucketPermission {
    pub bucket_id: Uuid,
    pub bucket_name: String,
    pub physical_bucket_name: String,
    pub region: String,
    pub provider_id: Uuid,
    pub platform_sse_key: String,
    pub can_read: bool,
    pub can_write: bool,
    #[serde(default)]
    pub is_deleting: bool,
}

#[derive(Clone)]
pub struct CredentialIdentity {
    pub organization_id: Option<Uuid>,
    pub credential_id: Uuid,
    pub prefix: String,
    pub bucket_permissions: Vec<BucketPermission>,
}

impl CredentialResolver {
    pub fn new(api_url: String, service_token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_url,
            service_token,
        }
    }

    async fn resolve(
        &self,
        access_key: &str,
    ) -> Result<Option<ResolvedCredential>, reqwest::Error> {
        let response = self
            .http
            .get(format!(
                "{}/internal/s3-access-tokens/resolve/{access_key}",
                self.api_url
            ))
            .header("x-cplane-token", &self.service_token)
            .send()
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        Ok(Some(response.error_for_status()?.json().await?))
    }

    pub async fn provider(&self, id: Uuid) -> Result<S3Provider, reqwest::Error> {
        self.http
            .get(format!(
                "{}/internal/s3-providers/{id}/credentials",
                self.api_url
            ))
            .header("x-cplane-token", &self.service_token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }
}

#[async_trait]
impl S3Auth for CredentialResolver {
    async fn get_secret_key(&self, access_key: &str) -> S3Result<SecretKey> {
        match self.resolve(access_key).await {
            Ok(Some(credential)) => Ok(SecretKey::from(credential.secret_access_key)),
            Ok(None) => Err(s3s::s3_error!(InvalidAccessKeyId)),
            Err(error) => {
                tracing::error!(%error, "credential resolver failed");
                Err(s3s::s3_error!(InternalError))
            }
        }
    }
}

#[async_trait]
impl S3Access for CredentialResolver {
    async fn check(&self, context: &mut S3AccessContext<'_>) -> S3Result<()> {
        let Some(credentials) = context.credentials() else {
            return Err(s3s::s3_error!(AccessDenied, "Signature is required"));
        };
        match self.resolve(&credentials.access_key).await {
            Ok(Some(identity)) => {
                if let Some(bucket) = context.s3_path().get_bucket_name() {
                    let permission = identity
                        .bucket_permissions
                        .iter()
                        .find(|permission| permission.bucket_name == bucket)
                        .ok_or_else(|| s3s::s3_error!(AccessDenied))?;
                    if is_write_operation(context.s3_op().name())
                        && (permission.is_deleting || !permission.can_write)
                        || !is_write_operation(context.s3_op().name()) && !permission.can_read
                    {
                        return Err(s3s::s3_error!(AccessDenied));
                    }
                } else if context.s3_op().name() != "ListBuckets" {
                    return Err(s3s::s3_error!(AccessDenied));
                }
                context.extensions_mut().insert(CredentialIdentity {
                    organization_id: identity.organization_id,
                    credential_id: identity.credential_id,
                    prefix: identity.prefix,
                    bucket_permissions: identity.bucket_permissions,
                });
                Ok(())
            }
            Ok(None) => Err(s3s::s3_error!(AccessDenied)),
            Err(error) => {
                tracing::error!(%error, "credential access check failed");
                Err(s3s::s3_error!(InternalError))
            }
        }
    }
}

fn is_write_operation(operation: &str) -> bool {
    matches!(
        operation,
        "AbortMultipartUpload"
            | "CompleteMultipartUpload"
            | "CopyObject"
            | "CreateMultipartUpload"
            | "DeleteObject"
            | "DeleteObjects"
            | "PutObject"
            | "UploadPart"
            | "UploadPartCopy"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        extract::Path,
        http::{HeaderMap, StatusCode},
        routing::get,
    };
    use s3s::auth::S3Auth;
    use serde_json::json;

    async fn resolve(
        Path(access_key): Path<String>,
        headers: HeaderMap,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        if !matches!(access_key.as_str(), "VALID" | "SERVICE")
            || headers.get("x-cplane-token").is_none_or(|v| v != "service")
        {
            return Err(StatusCode::NOT_FOUND);
        }
        Ok(Json(json!({
            "organization_id": (access_key == "VALID").then(Uuid::nil),
            "credential_id": Uuid::nil(),
            "prefix": "backups/production/",
            "bucket_permissions": [{
                "bucket_id": Uuid::nil(),
                "bucket_name": "uploads",
                "physical_bucket_name": "cp-00000000000000000000000000000000",
                "region": "local",
                "provider_id": Uuid::nil(),
                "platform_sse_key": "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=",
                "can_read": true,
                "can_write": true,
                "is_deleting": false
            }],
            "secret_access_key": "secret"
        })))
    }

    async fn provider(
        Path(id): Path<Uuid>,
        headers: HeaderMap,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        if headers.get("x-cplane-token").is_none_or(|v| v != "service") {
            return Err(StatusCode::UNAUTHORIZED);
        }
        if id != Uuid::nil() {
            return Err(StatusCode::NOT_FOUND);
        }
        Ok(Json(json!({
            "endpoint_url": "http://provider",
            "provider_region": "local",
            "access_key_id": "provider-access",
            "secret_access_key": "provider-secret",
            "session_token": null
        })))
    }

    #[tokio::test]
    async fn resolves_sigv4_secret_by_access_key() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new()
                    .route(
                        "/internal/s3-access-tokens/resolve/{access_key}",
                        get(resolve),
                    )
                    .route("/internal/s3-providers/{id}/credentials", get(provider)),
            )
            .await
            .unwrap();
        });
        let resolver = CredentialResolver::new(format!("http://{address}"), "service".into());
        assert_eq!(
            resolver.get_secret_key("VALID").await.unwrap().expose(),
            "secret"
        );
        assert_eq!(
            resolver.get_secret_key("SERVICE").await.unwrap().expose(),
            "secret"
        );
        assert_eq!(
            resolver.provider(Uuid::nil()).await.unwrap().endpoint_url,
            "http://provider"
        );
        assert!(resolver.get_secret_key("INVALID").await.is_err());
        server.abort();
    }

    #[test]
    fn classifies_mutating_operations() {
        assert!(is_write_operation("PutObject"));
        assert!(is_write_operation("UploadPart"));
        assert!(!is_write_operation("GetObject"));
        assert!(!is_write_operation("ListObjectsV2"));
    }
}
