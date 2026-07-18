use crate::models::*;
use dioxus::prelude::*;

#[get("/api/infrastructure/regions")]
pub async fn list_regions() -> Result<Vec<Region>> {
    server::list_regions().await
}

#[post("/api/infrastructure/regions", headers: dioxus::fullstack::HeaderMap)]
pub async fn create_region(
    slug: String,
    display_name: String,
    status: String,
    s3_provider_id: Option<String>,
) -> Result<()> {
    server::create_region(&headers, slug, display_name, status, s3_provider_id).await
}

#[patch("/api/infrastructure/regions/{id}", headers: dioxus::fullstack::HeaderMap)]
pub async fn update_region(
    id: String,
    slug: String,
    display_name: String,
    status: String,
    s3_provider_id: Option<String>,
) -> Result<()> {
    server::update_region(&headers, id, slug, display_name, status, s3_provider_id).await
}

#[delete("/api/infrastructure/regions/{id}", headers: dioxus::fullstack::HeaderMap)]
pub async fn delete_region(id: String) -> Result<()> {
    server::delete_resource(&headers, "regions", "region", id).await
}

#[get("/api/infrastructure/clusters")]
pub async fn list_clusters() -> Result<Vec<Cluster>> {
    server::list_clusters().await
}

#[post("/api/infrastructure/clusters", headers: dioxus::fullstack::HeaderMap)]
pub async fn create_cluster(
    region_id: String,
    name: String,
    slug: String,
) -> Result<JoinCredential> {
    server::create_cluster(&headers, region_id, name, slug).await
}

#[patch("/api/infrastructure/clusters/{id}", headers: dioxus::fullstack::HeaderMap)]
pub async fn update_cluster(
    id: String,
    region_id: String,
    name: String,
    slug: String,
    agent_id: Option<String>,
    agent_endpoint: Option<String>,
    status: String,
    health_status: String,
    capacity_allocatable: i32,
    capacity_used: i32,
) -> Result<()> {
    server::update_cluster(
        &headers,
        id,
        region_id,
        name,
        slug,
        agent_id,
        agent_endpoint,
        status,
        health_status,
        capacity_allocatable,
        capacity_used,
    )
    .await
}

#[delete("/api/infrastructure/clusters/{id}", headers: dioxus::fullstack::HeaderMap)]
pub async fn delete_cluster(id: String) -> Result<()> {
    server::delete_resource(&headers, "clusters", "cluster", id).await
}

#[post("/api/infrastructure/clusters/{id}/join-credentials", headers: dioxus::fullstack::HeaderMap)]
pub async fn issue_join_credential(id: String) -> Result<JoinCredential> {
    server::issue_join_credential(&headers, id).await
}

#[get("/api/infrastructure/s3-providers")]
pub async fn list_s3_providers() -> Result<Vec<S3Provider>> {
    server::list_s3_providers().await
}

#[post("/api/infrastructure/s3-providers", headers: dioxus::fullstack::HeaderMap)]
pub async fn create_s3_provider(
    provider_type: String,
    endpoint_url: String,
    provider_region: String,
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
    is_active: bool,
) -> Result<()> {
    server::create_s3_provider(
        &headers,
        provider_type,
        endpoint_url,
        provider_region,
        S3Credentials {
            access_key_id,
            secret_access_key,
            session_token,
        },
        is_active,
    )
    .await
}

#[patch("/api/infrastructure/s3-providers/{id}", headers: dioxus::fullstack::HeaderMap)]
pub async fn update_s3_provider(
    id: String,
    provider_type: String,
    endpoint_url: String,
    provider_region: String,
    access_key_id: Option<String>,
    secret_access_key: Option<String>,
    session_token: Option<String>,
    is_active: bool,
) -> Result<()> {
    let credentials = match (access_key_id, secret_access_key) {
        (Some(access_key_id), Some(secret_access_key)) => Some(S3Credentials {
            access_key_id,
            secret_access_key,
            session_token,
        }),
        (None, None) => None,
        _ => {
            return Err(dioxus::CapturedError::msg(
                "Access key and secret key must be replaced together",
            ));
        }
    };
    server::update_s3_provider(
        &headers,
        id,
        provider_type,
        endpoint_url,
        provider_region,
        credentials,
        is_active,
    )
    .await
}

#[delete("/api/infrastructure/s3-providers/{id}", headers: dioxus::fullstack::HeaderMap)]
pub async fn delete_s3_provider(id: String) -> Result<()> {
    server::delete_s3_provider(&headers, id).await
}

#[get("/api/organizations")]
pub async fn list_organizations() -> Result<Vec<Organization>> {
    server::list_organizations().await
}

#[patch("/api/organizations/{id}", headers: dioxus::fullstack::HeaderMap)]
pub async fn update_organization(id: String, name: String) -> Result<()> {
    server::update_organization(&headers, id, name).await
}

#[delete("/api/organizations/{id}", headers: dioxus::fullstack::HeaderMap)]
pub async fn delete_organization(id: String) -> Result<()> {
    server::delete_resource(&headers, "organization", "organization", id).await
}

#[get("/api/api-keys")]
pub async fn list_api_keys() -> Result<Vec<ApiKey>> {
    server::list_api_keys().await
}

#[delete("/api/api-keys/{id}", headers: dioxus::fullstack::HeaderMap)]
pub async fn delete_api_key(id: String) -> Result<()> {
    server::delete_resource(&headers, "api_keys", "api_key", id).await
}

#[get("/api/audit-logs")]
pub async fn list_audit_logs() -> Result<Vec<AuditLog>> {
    server::list_audit_logs().await
}

#[cfg(feature = "server")]
pub mod server {
    use super::*;
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use chrono::{Duration, Utc};
    use dioxus::{CapturedError, fullstack::HeaderMap};
    use redis::AsyncCommands;
    use reqwest::Client;
    use sea_orm::{
        ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, QueryResult, Statement,
        TransactionTrait, Value,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::env;
    use tokio::sync::OnceCell;
    use uuid::Uuid;

    const S3_PROVIDER_CACHE_PREFIX: &str = "cplane:s3-provider:";
    const S3_ACCESS_TOKEN_CACHE_PREFIX: &str = "cplane:s3-access-token:";
    const S3_ACCESS_TOKEN_CACHE_GENERATION: &str = "cplane:s3-access-token-generation";
    const S3_ACCESS_TOKEN_CACHE_TTL_SECONDS: u64 = 86_400;
    static DATABASE: OnceCell<DatabaseConnection> = OnceCell::const_new();

    pub async fn initialize() -> Result<()> {
        required(
            env::var("CPLANE_SERVICE_TOKEN").map_err(CapturedError::from_display)?,
            "CPLANE_SERVICE_TOKEN",
        )?;
        database().await?;
        sync_provider_cache().await?;
        bootstrap_registry_storage().await?;
        Ok(())
    }

    async fn database() -> Result<&'static DatabaseConnection> {
        DATABASE
            .get_or_try_init(|| async {
                let url = env::var("ADMIN_DATABASE_URL").map_err(CapturedError::from_display)?;
                Database::connect(url)
                    .await
                    .map_err(CapturedError::from_display)
            })
            .await
    }

    fn statement(sql: &str, values: Vec<Value>) -> Statement {
        Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
    }

    fn required(value: String, name: &str) -> Result<String> {
        let value = value.trim().to_string();
        if value.is_empty() {
            return Err(CapturedError::msg(format!("{name} is required")));
        }
        Ok(value)
    }

    fn optional_env(name: &str) -> Option<String> {
        env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    fn validate_region_slug(slug: String) -> Result<String> {
        let slug = required(slug, "slug")?.to_ascii_lowercase();
        if matches!(slug.as_str(), "default" | "global" | "system")
            || !slug
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(CapturedError::msg("Invalid or reserved region slug"));
        }
        Ok(slug)
    }

    fn validate_choice(value: String, allowed: &[&str], name: &str) -> Result<String> {
        if allowed.contains(&value.as_str()) {
            Ok(value)
        } else {
            Err(CapturedError::msg(format!("Invalid {name}")))
        }
    }

    fn request_identity(headers: &HeaderMap) -> (String, String) {
        let source_ip = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()))
            .unwrap_or("local")
            .trim()
            .to_string();
        let actor = headers
            .get("x-authenticated-user")
            .and_then(|v| v.to_str().ok())
            .unwrap_or(&source_ip)
            .to_string();
        (actor, source_ip)
    }

    async fn audit(
        connection: &impl ConnectionTrait,
        headers: &HeaderMap,
        action: &str,
        resource_type: &str,
        resource_id: Option<&str>,
        changes: serde_json::Value,
    ) -> Result<()> {
        let (actor, source_ip) = request_identity(headers);
        connection
            .execute(statement(
                "INSERT INTO infrastructure_audit_log (id, actor_identifier, source_ip, action, resource_type, resource_id, changes) VALUES ($1::uuid, $2, $3, $4, $5, $6::uuid, $7::jsonb)",
                vec![
                    Uuid::new_v4().to_string().into(),
                    actor.into(),
                    source_ip.into(),
                    action.to_string().into(),
                    resource_type.to_string().into(),
                    resource_id.map(ToString::to_string).into(),
                    changes.to_string().into(),
                ],
            ))
            .await
            .map_err(CapturedError::from_display)?;
        Ok(())
    }

    fn text(row: &QueryResult, name: &str) -> Result<String> {
        row.try_get("", name).map_err(CapturedError::from_display)
    }

    fn optional_text(row: &QueryResult, name: &str) -> Result<Option<String>> {
        row.try_get("", name).map_err(CapturedError::from_display)
    }

    pub async fn list_regions() -> Result<Vec<Region>> {
        let rows = database().await?.query_all(statement(
            "SELECT id::text, slug, display_name, status::text, s3_provider_id::text FROM regions ORDER BY display_name",
            vec![],
        )).await.map_err(CapturedError::from_display)?;
        rows.iter()
            .map(|row| {
                Ok(Region {
                    id: text(row, "id")?,
                    slug: text(row, "slug")?,
                    display_name: text(row, "display_name")?,
                    status: text(row, "status")?,
                    s3_provider_id: optional_text(row, "s3_provider_id")?,
                })
            })
            .collect()
    }

    pub async fn create_region(
        headers: &HeaderMap,
        slug: String,
        display_name: String,
        status: String,
        s3_provider_id: Option<String>,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let slug = validate_region_slug(slug)?;
        let display_name = required(display_name, "display name")?;
        let status = validate_choice(status, &["active", "inactive", "maintenance"], "status")?;
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        tx.execute(statement(
            "INSERT INTO regions (id, slug, display_name, status, s3_provider_id) VALUES ($1::uuid, $2, $3, $4::region_status, $5::uuid)",
            vec![id.clone().into(), slug.clone().into(), display_name.clone().into(), status.clone().into(), s3_provider_id.clone().into()],
        )).await.map_err(CapturedError::from_display)?;
        audit(&tx, headers, "create", "region", Some(&id), json!({"slug": slug, "display_name": display_name, "status": status, "s3_provider_id": s3_provider_id})).await?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        Ok(())
    }

    pub async fn update_region(
        headers: &HeaderMap,
        id: String,
        slug: String,
        display_name: String,
        status: String,
        s3_provider_id: Option<String>,
    ) -> Result<()> {
        let slug = validate_region_slug(slug)?;
        let display_name = required(display_name, "display name")?;
        let status = validate_choice(status, &["active", "inactive", "maintenance"], "status")?;
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        let result = tx.execute(statement(
            "UPDATE regions SET slug=$2, display_name=$3, status=$4::region_status, s3_provider_id=$5::uuid, updated_at=now() WHERE id=$1::uuid",
            vec![id.clone().into(), slug.clone().into(), display_name.clone().into(), status.clone().into(), s3_provider_id.clone().into()],
        )).await.map_err(CapturedError::from_display)?;
        if result.rows_affected() == 0 {
            return Err(CapturedError::msg("Region not found"));
        }
        audit(&tx, headers, "update", "region", Some(&id), json!({"slug": slug, "display_name": display_name, "status": status, "s3_provider_id": s3_provider_id})).await?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        invalidate_access_token_cache().await?;
        Ok(())
    }

    pub async fn list_clusters() -> Result<Vec<Cluster>> {
        let rows = database().await?.query_all(statement(
            "SELECT id::text, name, slug, region_id::text, agent_id, agent_endpoint, status::text, health_status::text, capacity_allocatable, capacity_used FROM clusters ORDER BY name",
            vec![],
        )).await.map_err(CapturedError::from_display)?;
        rows.iter()
            .map(|row| {
                Ok(Cluster {
                    id: text(row, "id")?,
                    name: text(row, "name")?,
                    slug: text(row, "slug")?,
                    region_id: text(row, "region_id")?,
                    agent_id: optional_text(row, "agent_id")?,
                    agent_endpoint: optional_text(row, "agent_endpoint")?,
                    status: text(row, "status")?,
                    health_status: text(row, "health_status")?,
                    capacity_allocatable: row
                        .try_get("", "capacity_allocatable")
                        .map_err(CapturedError::from_display)?,
                    capacity_used: row
                        .try_get("", "capacity_used")
                        .map_err(CapturedError::from_display)?,
                })
            })
            .collect()
    }

    fn join_token() -> (String, String) {
        let token = format!("cj_{}", Uuid::new_v4().simple());
        let pepper = env::var("CLUSTER_JOIN_TOKEN_PEPPER").unwrap_or_default();
        let hash = hex::encode(Sha256::digest(format!("{token}.{pepper}")));
        (token, hash)
    }

    async fn insert_join_credential(
        connection: &impl ConnectionTrait,
        cluster_id: &str,
    ) -> Result<JoinCredential> {
        let (token, hash) = join_token();
        let expires_at = (Utc::now() + Duration::minutes(15)).to_rfc3339();
        connection.execute(statement(
            "INSERT INTO cluster_join_credentials (id, cluster_id, token_hash, expires_at) VALUES ($1::uuid, $2::uuid, $3, $4::timestamptz)",
            vec![Uuid::new_v4().to_string().into(), cluster_id.to_string().into(), hash.into(), expires_at.clone().into()],
        )).await.map_err(CapturedError::from_display)?;
        Ok(JoinCredential { token, expires_at })
    }

    pub async fn create_cluster(
        headers: &HeaderMap,
        region_id: String,
        name: String,
        slug: String,
    ) -> Result<JoinCredential> {
        let id = Uuid::new_v4().to_string();
        let name = required(name, "name")?;
        let slug = required(slug, "slug")?.to_ascii_lowercase();
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        tx.execute(statement(
            "INSERT INTO clusters (id, region_id, slug, name, health_status) VALUES ($1::uuid, $2::uuid, $3, $4, 'offline')",
            vec![id.clone().into(), region_id.clone().into(), slug.clone().into(), name.clone().into()],
        )).await.map_err(CapturedError::from_display)?;
        let credential = insert_join_credential(&tx, &id).await?;
        audit(
            &tx,
            headers,
            "create",
            "cluster",
            Some(&id),
            json!({"region_id": region_id, "slug": slug, "name": name}),
        )
        .await?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        Ok(credential)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_cluster(
        headers: &HeaderMap,
        id: String,
        region_id: String,
        name: String,
        slug: String,
        agent_id: Option<String>,
        agent_endpoint: Option<String>,
        status: String,
        health_status: String,
        capacity_allocatable: i32,
        capacity_used: i32,
    ) -> Result<()> {
        let name = required(name, "name")?;
        let slug = required(slug, "slug")?.to_ascii_lowercase();
        let status = validate_choice(
            status,
            &[
                "pending",
                "bootstrapping",
                "healthy",
                "draining",
                "offline",
                "removed",
            ],
            "status",
        )?;
        let health_status = validate_choice(
            health_status,
            &["healthy", "degraded", "offline"],
            "health status",
        )?;
        if capacity_allocatable < 0 || capacity_used < 0 {
            return Err(CapturedError::msg("Capacity cannot be negative"));
        }
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        let result = tx.execute(statement(
            "UPDATE clusters SET region_id=$2::uuid, name=$3, slug=$4, agent_id=$5, agent_endpoint=$6, status=$7::cluster_status, health_status=$8::cluster_health_status, capacity_allocatable=$9, capacity_used=$10, updated_at=now() WHERE id=$1::uuid",
            vec![id.clone().into(), region_id.clone().into(), name.clone().into(), slug.clone().into(), agent_id.clone().into(), agent_endpoint.clone().into(), status.clone().into(), health_status.clone().into(), capacity_allocatable.into(), capacity_used.into()],
        )).await.map_err(CapturedError::from_display)?;
        if result.rows_affected() == 0 {
            return Err(CapturedError::msg("Cluster not found"));
        }
        audit(&tx, headers, "update", "cluster", Some(&id), json!({"region_id": region_id, "name": name, "slug": slug, "agent_id": agent_id, "agent_endpoint": agent_endpoint, "status": status, "health_status": health_status, "capacity_allocatable": capacity_allocatable, "capacity_used": capacity_used})).await?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        Ok(())
    }

    pub async fn issue_join_credential(headers: &HeaderMap, id: String) -> Result<JoinCredential> {
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        tx.execute(statement("UPDATE cluster_join_credentials SET revoked_at=now(), revoked_reason='reissued', updated_at=now() WHERE cluster_id=$1::uuid AND used_at IS NULL AND revoked_at IS NULL AND expires_at > now()", vec![id.clone().into()])).await.map_err(CapturedError::from_display)?;
        let credential = insert_join_credential(&tx, &id).await?;
        audit(
            &tx,
            headers,
            "issue_join_credential",
            "cluster",
            Some(&id),
            json!({"expires_at": credential.expires_at}),
        )
        .await?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        Ok(credential)
    }

    pub async fn list_s3_providers() -> Result<Vec<S3Provider>> {
        let rows = database().await?.query_all(statement("SELECT id::text, provider_type::text, endpoint_url, provider_region, is_active FROM s3_providers ORDER BY endpoint_url", vec![])).await.map_err(CapturedError::from_display)?;
        rows.iter()
            .map(|row| {
                Ok(S3Provider {
                    id: text(row, "id")?,
                    provider_type: text(row, "provider_type")?,
                    endpoint_url: text(row, "endpoint_url")?,
                    provider_region: optional_text(row, "provider_region")?,
                    is_active: row
                        .try_get("", "is_active")
                        .map_err(CapturedError::from_display)?,
                })
            })
            .collect()
    }

    #[derive(Deserialize)]
    struct OpenBaoData {
        data: OpenBaoSecret,
    }
    #[derive(Deserialize)]
    struct OpenBaoSecret {
        data: S3Credentials,
    }

    fn openbao_config() -> Result<(String, String)> {
        let address = env::var("OPENBAO_ADDR").map_err(CapturedError::from_display)?;
        let token = env::var("OPENBAO_TOKEN").map_err(CapturedError::from_display)?;
        Ok((address.trim_end_matches('/').to_string(), token))
    }

    async fn openbao_read(id: &str) -> Result<S3Credentials> {
        let (address, token) = openbao_config()?;
        let response = Client::new()
            .get(format!(
                "{address}/v1/cplane/data/platform/s3/providers/{id}"
            ))
            .header("X-Vault-Token", token)
            .send()
            .await
            .map_err(CapturedError::from_display)?;
        if !response.status().is_success() {
            return Err(CapturedError::msg(format!(
                "OpenBao read failed: {}",
                response.status()
            )));
        }
        response
            .json::<OpenBaoData>()
            .await
            .map(|value| value.data.data)
            .map_err(CapturedError::from_display)
    }

    async fn openbao_write(id: &str, credentials: &S3Credentials) -> Result<()> {
        let (address, token) = openbao_config()?;
        let response = Client::new()
            .post(format!(
                "{address}/v1/cplane/data/platform/s3/providers/{id}"
            ))
            .header("X-Vault-Token", token)
            .json(&json!({"data": credentials}))
            .send()
            .await
            .map_err(CapturedError::from_display)?;
        if !response.status().is_success() {
            return Err(CapturedError::msg(format!(
                "OpenBao write failed: {}",
                response.status()
            )));
        }
        Ok(())
    }

    async fn openbao_delete(id: &str) -> Result<()> {
        let (address, token) = openbao_config()?;
        let response = Client::new()
            .delete(format!(
                "{address}/v1/cplane/data/platform/s3/providers/{id}"
            ))
            .header("X-Vault-Token", token)
            .send()
            .await
            .map_err(CapturedError::from_display)?;
        if !response.status().is_success() {
            return Err(CapturedError::msg(format!(
                "OpenBao delete failed: {}",
                response.status()
            )));
        }
        Ok(())
    }

    #[derive(Clone, Deserialize, Serialize)]
    pub struct S3AccessTokenSecret {
        pub secret_access_key: String,
    }

    #[derive(Deserialize)]
    struct OpenBaoAccessTokenData {
        data: OpenBaoAccessTokenSecret,
    }

    #[derive(Deserialize)]
    struct OpenBaoAccessTokenSecret {
        data: S3AccessTokenSecret,
    }

    #[derive(Clone, Deserialize, Serialize)]
    pub struct ResolvedS3BucketPermission {
        pub bucket_id: Uuid,
        pub bucket_name: String,
        pub physical_bucket_name: String,
        pub region: String,
        pub provider_id: Uuid,
        pub platform_sse_key: String,
        pub can_read: bool,
        pub can_write: bool,
    }

    #[derive(Clone, Deserialize, Serialize)]
    pub struct ResolvedS3AccessToken {
        pub organization_id: Option<Uuid>,
        pub project_id: Option<Uuid>,
        pub credential_id: Uuid,
        pub bucket_permissions: Vec<ResolvedS3BucketPermission>,
        pub secret_access_key: String,
    }

    #[derive(Serialize)]
    pub struct S3ProviderConnection {
        pub id: Uuid,
        #[serde(flatten)]
        pub credentials: S3Credentials,
        pub endpoint_url: String,
        pub provider_region: Option<String>,
        pub provider_type: String,
    }

    async fn access_token_secret_read(id: &str) -> Result<S3AccessTokenSecret> {
        let (address, token) = openbao_config()?;
        let response = Client::new()
            .get(format!(
                "{address}/v1/cplane/data/platform/s3/access-tokens/{id}"
            ))
            .header("X-Vault-Token", token)
            .send()
            .await
            .map_err(CapturedError::from_display)?;
        if !response.status().is_success() {
            return Err(CapturedError::msg(format!(
                "OpenBao read failed: {}",
                response.status()
            )));
        }
        response
            .json::<OpenBaoAccessTokenData>()
            .await
            .map(|value| value.data.data)
            .map_err(CapturedError::from_display)
    }

    async fn service_credential_secret_read(id: &str) -> Result<S3AccessTokenSecret> {
        let (address, token) = openbao_config()?;
        let response = Client::new()
            .get(format!(
                "{address}/v1/cplane/data/platform/s3/service-credentials/{id}"
            ))
            .header("X-Vault-Token", token)
            .send()
            .await
            .map_err(CapturedError::from_display)?;
        if !response.status().is_success() {
            return Err(CapturedError::msg(format!(
                "OpenBao read failed: {}",
                response.status()
            )));
        }
        response
            .json::<OpenBaoAccessTokenData>()
            .await
            .map(|value| value.data.data)
            .map_err(CapturedError::from_display)
    }

    #[derive(Deserialize)]
    struct OpenBaoBucketKeyData {
        data: OpenBaoBucketKeySecret,
    }

    #[derive(Deserialize)]
    struct OpenBaoBucketKeySecret {
        data: BucketKey,
    }

    #[derive(Deserialize, Serialize)]
    struct BucketKey {
        key: String,
    }

    async fn bucket_key_read(id: Uuid) -> Result<Option<String>> {
        let (address, token) = openbao_config()?;
        let response = Client::new()
            .get(format!("{address}/v1/cplane/data/storage/sse-c/{id}"))
            .header("X-Vault-Token", token)
            .send()
            .await
            .map_err(CapturedError::from_display)?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(CapturedError::msg(format!(
                "OpenBao read failed: {}",
                response.status()
            )));
        }
        response
            .json::<OpenBaoBucketKeyData>()
            .await
            .map(|value| Some(value.data.data.key))
            .map_err(CapturedError::from_display)
    }

    async fn bucket_key(id: Uuid) -> Result<String> {
        if let Some(key) = bucket_key_read(id).await? {
            return Ok(key);
        }
        let mut raw = [0; 32];
        raw[..16].copy_from_slice(Uuid::new_v4().as_bytes());
        raw[16..].copy_from_slice(Uuid::new_v4().as_bytes());
        let key = STANDARD.encode(raw);
        let (address, token) = openbao_config()?;
        let response = Client::new()
            .post(format!("{address}/v1/cplane/data/storage/sse-c/{id}"))
            .header("X-Vault-Token", token)
            .json(&json!({ "options": { "cas": 0 }, "data": { "key": key } }))
            .send()
            .await
            .map_err(CapturedError::from_display)?;
        if response.status().is_success() {
            return Ok(key);
        }
        bucket_key_read(id).await?.ok_or_else(|| {
            CapturedError::msg(format!("OpenBao write failed: {}", response.status()))
        })
    }

    async fn access_token_secret_write(id: &str, secret: &S3AccessTokenSecret) -> Result<()> {
        let (address, token) = openbao_config()?;
        let response = Client::new()
            .post(format!(
                "{address}/v1/cplane/data/platform/s3/access-tokens/{id}"
            ))
            .header("X-Vault-Token", token)
            .json(&json!({"data": secret}))
            .send()
            .await
            .map_err(CapturedError::from_display)?;
        if !response.status().is_success() {
            return Err(CapturedError::msg(format!(
                "OpenBao write failed: {}",
                response.status()
            )));
        }
        Ok(())
    }

    async fn service_credential_secret_write(id: &str, secret: &S3AccessTokenSecret) -> Result<()> {
        let (address, token) = openbao_config()?;
        let response = Client::new()
            .post(format!(
                "{address}/v1/cplane/data/platform/s3/service-credentials/{id}"
            ))
            .header("X-Vault-Token", token)
            .json(&json!({"data": secret}))
            .send()
            .await
            .map_err(CapturedError::from_display)?;
        if !response.status().is_success() {
            return Err(CapturedError::msg(format!(
                "OpenBao write failed: {}",
                response.status()
            )));
        }
        Ok(())
    }

    async fn access_token_secret_delete(id: &str) -> Result<()> {
        let (address, token) = openbao_config()?;
        let response = Client::new()
            .delete(format!(
                "{address}/v1/cplane/data/platform/s3/access-tokens/{id}"
            ))
            .header("X-Vault-Token", token)
            .send()
            .await
            .map_err(CapturedError::from_display)?;
        if !response.status().is_success() {
            return Err(CapturedError::msg(format!(
                "OpenBao delete failed: {}",
                response.status()
            )));
        }
        Ok(())
    }

    fn provider_cache_key(id: &str) -> String {
        format!("{S3_PROVIDER_CACHE_PREFIX}{id}")
    }

    async fn redis_connection() -> Result<redis::aio::MultiplexedConnection> {
        let url = env::var("REDIS_URL").map_err(CapturedError::from_display)?;
        let client = redis::Client::open(url).map_err(CapturedError::from_display)?;
        client
            .get_multiplexed_async_connection()
            .await
            .map_err(CapturedError::from_display)
    }

    async fn cache_credentials(id: &str, credentials: &S3Credentials) -> Result<()> {
        let mut connection = redis_connection().await?;
        let value = serde_json::to_string(credentials).map_err(CapturedError::from_display)?;
        let _: () = connection
            .set(provider_cache_key(id), value)
            .await
            .map_err(CapturedError::from_display)?;
        Ok(())
    }

    async fn cached_credentials(id: &str) -> Result<S3Credentials> {
        let mut connection = redis_connection().await?;
        let value: Option<String> = connection
            .get(provider_cache_key(id))
            .await
            .map_err(CapturedError::from_display)?;
        if let Some(value) = value {
            return serde_json::from_str(&value).map_err(CapturedError::from_display);
        }
        let credentials = openbao_read(id).await?;
        let value = serde_json::to_string(&credentials).map_err(CapturedError::from_display)?;
        let _: () = connection
            .set(provider_cache_key(id), value)
            .await
            .map_err(CapturedError::from_display)?;
        Ok(credentials)
    }

    async fn delete_cached_credentials(id: &str) -> Result<()> {
        let mut connection = redis_connection().await?;
        let _: usize = connection
            .del(provider_cache_key(id))
            .await
            .map_err(CapturedError::from_display)?;
        Ok(())
    }

    fn access_token_cache_key(access_key: &str, generation: u64) -> String {
        format!("{S3_ACCESS_TOKEN_CACHE_PREFIX}{generation}:{access_key}")
    }

    async fn cached_access_token(access_key: &str) -> Result<(u64, Option<ResolvedS3AccessToken>)> {
        let mut connection = redis_connection().await?;
        let generation: Option<u64> = connection
            .get(S3_ACCESS_TOKEN_CACHE_GENERATION)
            .await
            .map_err(CapturedError::from_display)?;
        let generation = generation.unwrap_or(0);
        let value: Option<String> = connection
            .get(access_token_cache_key(access_key, generation))
            .await
            .map_err(CapturedError::from_display)?;
        let token = value
            .map(|value| serde_json::from_str(&value).map_err(CapturedError::from_display))
            .transpose()?;
        Ok((generation, token))
    }

    async fn cache_access_token(
        access_key: &str,
        generation: u64,
        token: &ResolvedS3AccessToken,
    ) -> Result<()> {
        let mut connection = redis_connection().await?;
        let value = serde_json::to_string(token).map_err(CapturedError::from_display)?;
        let _: () = connection
            .set_ex(
                access_token_cache_key(access_key, generation),
                value,
                S3_ACCESS_TOKEN_CACHE_TTL_SECONDS,
            )
            .await
            .map_err(CapturedError::from_display)?;
        Ok(())
    }

    async fn invalidate_access_token_cache() -> Result<()> {
        let mut connection = redis_connection().await?;
        let _: u64 = connection
            .incr(S3_ACCESS_TOKEN_CACHE_GENERATION, 1)
            .await
            .map_err(CapturedError::from_display)?;
        Ok(())
    }

    async fn sync_provider_cache() -> Result<()> {
        let rows = database()
            .await?
            .query_all(statement("SELECT id::text FROM s3_providers", vec![]))
            .await
            .map_err(CapturedError::from_display)?;
        for row in rows {
            let id = text(&row, "id")?;
            cache_credentials(&id, &openbao_read(&id).await?).await?;
        }
        Ok(())
    }

    async fn bootstrap_registry_storage() -> Result<()> {
        let Some(provider_id) = optional_env("REGISTRY_STORAGE_S3_PROVIDER_ID") else {
            return Ok(());
        };
        let provider_id = Uuid::parse_str(&provider_id).map_err(CapturedError::from_display)?;
        let bucket_name = required(
            env::var("REGISTRY_STORAGE_S3_BUCKET").map_err(CapturedError::from_display)?,
            "REGISTRY_STORAGE_S3_BUCKET",
        )?;
        let physical_bucket_name = optional_env("REGISTRY_STORAGE_S3_PHYSICAL_BUCKET")
            .unwrap_or_else(|| bucket_name.clone());
        let access_key_id = required(
            env::var("REGISTRY_STORAGE_S3_ACCESSKEY").map_err(CapturedError::from_display)?,
            "REGISTRY_STORAGE_S3_ACCESSKEY",
        )?;
        let secret_access_key = required(
            env::var("REGISTRY_STORAGE_S3_SECRETKEY").map_err(CapturedError::from_display)?,
            "REGISTRY_STORAGE_S3_SECRETKEY",
        )?;
        if secret_access_key.len() < 32 {
            return Err(CapturedError::msg(
                "REGISTRY_STORAGE_S3_SECRETKEY must be at least 32 characters",
            ));
        }

        let database = database().await?;
        if database
            .query_one(statement(
                "SELECT id FROM s3_providers WHERE id=$1::uuid AND is_active=true",
                vec![provider_id.into()],
            ))
            .await
            .map_err(CapturedError::from_display)?
            .is_none()
        {
            return Err(CapturedError::msg(
                "REGISTRY_STORAGE_S3_PROVIDER_ID does not identify an active provider",
            ));
        }
        if database
            .query_one(statement(
                "SELECT id FROM storage_access_token WHERE access_key_id=$1 AND revoked_at IS NULL",
                vec![access_key_id.clone().into()],
            ))
            .await
            .map_err(CapturedError::from_display)?
            .is_some()
        {
            return Err(CapturedError::msg(
                "registry access key conflicts with a tenant storage credential",
            ));
        }

        let row = database
            .query_one(statement(
                "INSERT INTO registry_storage (id, service, provider_id, bucket_name, physical_bucket_name, access_key_id) VALUES ($1::uuid, 'distribution', $2::uuid, $3, $4, $5) ON CONFLICT (service) DO UPDATE SET provider_id=EXCLUDED.provider_id, bucket_name=EXCLUDED.bucket_name, physical_bucket_name=EXCLUDED.physical_bucket_name, access_key_id=EXCLUDED.access_key_id, updated_at=NOW() RETURNING id",
                vec![
                    Uuid::new_v4().into(),
                    provider_id.into(),
                    bucket_name.into(),
                    physical_bucket_name.into(),
                    access_key_id.into(),
                ],
            ))
            .await
            .map_err(CapturedError::from_display)?
            .ok_or_else(|| CapturedError::msg("registry storage bootstrap returned no row"))?;
        let id: Uuid = row.try_get("", "id").map_err(CapturedError::from_display)?;
        let secret = S3AccessTokenSecret { secret_access_key };
        let secret_changed = match service_credential_secret_read(&id.to_string()).await {
            Ok(stored) => stored.secret_access_key != secret.secret_access_key,
            Err(_) => true,
        };
        if secret_changed {
            service_credential_secret_write(&id.to_string(), &secret).await?;
        }
        bucket_key(id).await?;
        invalidate_access_token_cache().await?;
        Ok(())
    }

    pub async fn create_s3_provider(
        headers: &HeaderMap,
        provider_type: String,
        endpoint_url: String,
        provider_region: String,
        credentials: S3Credentials,
        is_active: bool,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let provider_type =
            validate_choice(provider_type, &["aws_s3", "cloudflare_r2"], "provider type")?;
        let endpoint_url = required(endpoint_url, "endpoint URL")?;
        let provider_region = required(provider_region, "provider region")?;
        required(credentials.access_key_id.clone(), "access key ID")?;
        required(credentials.secret_access_key.clone(), "secret access key")?;
        openbao_write(&id, &credentials).await?;
        let database_result: Result<()> = async {
            let tx = database()
                .await?
                .begin()
                .await
                .map_err(CapturedError::from_display)?;
            tx.execute(statement("INSERT INTO s3_providers (id, provider_type, endpoint_url, provider_region, is_active) VALUES ($1::uuid, $2::s3_provider_type, $3, $4, $5)", vec![id.clone().into(), provider_type.clone().into(), endpoint_url.clone().into(), provider_region.clone().into(), is_active.into()])).await.map_err(CapturedError::from_display)?;
            audit(&tx, headers, "create", "s3_provider", Some(&id), json!({"provider_type": provider_type, "endpoint_url": endpoint_url, "provider_region": provider_region, "is_active": is_active})).await?;
            tx.commit().await.map_err(CapturedError::from_display)?;
            Ok(())
        }.await;
        if let Err(error) = database_result {
            let _ = openbao_delete(&id).await;
            return Err(error);
        }
        cache_credentials(&id, &credentials).await
    }

    pub async fn update_s3_provider(
        headers: &HeaderMap,
        id: String,
        provider_type: String,
        endpoint_url: String,
        provider_region: String,
        credentials: Option<S3Credentials>,
        is_active: bool,
    ) -> Result<()> {
        let provider_type =
            validate_choice(provider_type, &["aws_s3", "cloudflare_r2"], "provider type")?;
        let endpoint_url = required(endpoint_url, "endpoint URL")?;
        let provider_region = required(provider_region, "provider region")?;
        let previous_credentials = cached_credentials(&id).await?;
        if let Some(replacement) = credentials.as_ref() {
            openbao_write(&id, replacement).await?;
        }
        let database_result: Result<()> = async {
            let tx = database()
                .await?
                .begin()
                .await
                .map_err(CapturedError::from_display)?;
            let result = tx.execute(statement("UPDATE s3_providers SET provider_type=$2::s3_provider_type, endpoint_url=$3, provider_region=$4, is_active=$5, updated_at=now() WHERE id=$1::uuid", vec![id.clone().into(), provider_type.clone().into(), endpoint_url.clone().into(), provider_region.clone().into(), is_active.into()])).await.map_err(CapturedError::from_display)?;
            if result.rows_affected() == 0 {
                return Err(CapturedError::msg("S3 provider not found"));
            }
            audit(&tx, headers, "update", "s3_provider", Some(&id), json!({"provider_type": provider_type, "endpoint_url": endpoint_url, "provider_region": provider_region, "is_active": is_active, "credentials_rotated": credentials.is_some()})).await?;
            tx.commit().await.map_err(CapturedError::from_display)?;
            Ok(())
        }.await;
        if let Err(error) = database_result {
            if credentials.is_some() {
                let _ = openbao_write(&id, &previous_credentials).await;
            }
            return Err(error);
        }
        cache_credentials(&id, credentials.as_ref().unwrap_or(&previous_credentials)).await?;
        invalidate_access_token_cache().await
    }

    pub async fn delete_s3_provider(headers: &HeaderMap, id: String) -> Result<()> {
        let old = cached_credentials(&id).await?;
        openbao_delete(&id).await?;
        let result = async {
            let tx = database()
                .await?
                .begin()
                .await
                .map_err(CapturedError::from_display)?;
            let deleted = tx
                .execute(statement(
                    "DELETE FROM s3_providers WHERE id=$1::uuid",
                    vec![id.clone().into()],
                ))
                .await
                .map_err(CapturedError::from_display)?;
            if deleted.rows_affected() == 0 {
                return Err(CapturedError::msg("S3 provider not found"));
            }
            audit(&tx, headers, "delete", "s3_provider", Some(&id), json!({})).await?;
            tx.commit().await.map_err(CapturedError::from_display)?;
            Ok(())
        }
        .await;
        if let Err(error) = result {
            let _ = openbao_write(&id, &old).await;
            return Err(error);
        }
        delete_cached_credentials(&id).await?;
        invalidate_access_token_cache().await
    }

    fn authorize_service(
        headers: &dioxus::server::axum::http::HeaderMap,
    ) -> std::result::Result<(), dioxus::server::axum::http::StatusCode> {
        let expected = env::var("CPLANE_SERVICE_TOKEN")
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        let supplied = headers
            .get("x-cplane-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if expected.is_empty() || supplied != expected {
            return Err(dioxus::server::axum::http::StatusCode::UNAUTHORIZED);
        }
        Ok(())
    }

    pub async fn credentials_handler(
        dioxus::server::axum::extract::Path(id): dioxus::server::axum::extract::Path<String>,
        headers: dioxus::server::axum::http::HeaderMap,
    ) -> std::result::Result<
        dioxus::server::axum::Json<S3ProviderConnection>,
        dioxus::server::axum::http::StatusCode,
    > {
        authorize_service(&headers)?;
        let id = Uuid::parse_str(&id)
            .map_err(|_| dioxus::server::axum::http::StatusCode::BAD_REQUEST)?;
        Ok(dioxus::server::axum::Json(provider_connection(id).await?))
    }

    pub async fn ensure_bucket_sse_key_handler(
        dioxus::server::axum::extract::Path(id): dioxus::server::axum::extract::Path<String>,
        headers: dioxus::server::axum::http::HeaderMap,
    ) -> std::result::Result<
        dioxus::server::axum::http::StatusCode,
        dioxus::server::axum::http::StatusCode,
    > {
        authorize_service(&headers)?;
        let id = Uuid::parse_str(&id)
            .map_err(|_| dioxus::server::axum::http::StatusCode::BAD_REQUEST)?;
        bucket_key(id)
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(dioxus::server::axum::http::StatusCode::NO_CONTENT)
    }

    async fn provider_connection(
        id: Uuid,
    ) -> std::result::Result<S3ProviderConnection, dioxus::server::axum::http::StatusCode> {
        let provider = database()
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
            .query_one(statement(
                "SELECT endpoint_url, provider_region, provider_type::text FROM s3_providers WHERE id=$1::uuid AND is_active=true",
                vec![id.into()],
            ))
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(dioxus::server::axum::http::StatusCode::NOT_FOUND)?;
        Ok(S3ProviderConnection {
            id,
            credentials: cached_credentials(&id.to_string())
                .await
                .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?,
            endpoint_url: provider
                .try_get("", "endpoint_url")
                .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?,
            provider_region: provider
                .try_get("", "provider_region")
                .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?,
            provider_type: provider
                .try_get("", "provider_type")
                .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?,
        })
    }

    pub async fn store_access_token_secret_handler(
        dioxus::server::axum::extract::Path(id): dioxus::server::axum::extract::Path<String>,
        headers: dioxus::server::axum::http::HeaderMap,
        dioxus::server::axum::Json(secret): dioxus::server::axum::Json<S3AccessTokenSecret>,
    ) -> std::result::Result<
        dioxus::server::axum::http::StatusCode,
        dioxus::server::axum::http::StatusCode,
    > {
        authorize_service(&headers)?;
        Uuid::parse_str(&id).map_err(|_| dioxus::server::axum::http::StatusCode::BAD_REQUEST)?;
        if secret.secret_access_key.len() < 32 {
            return Err(dioxus::server::axum::http::StatusCode::BAD_REQUEST);
        }
        access_token_secret_write(&id, &secret)
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        invalidate_access_token_cache()
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(dioxus::server::axum::http::StatusCode::NO_CONTENT)
    }

    pub async fn delete_access_token_secret_handler(
        dioxus::server::axum::extract::Path(id): dioxus::server::axum::extract::Path<String>,
        headers: dioxus::server::axum::http::HeaderMap,
    ) -> std::result::Result<
        dioxus::server::axum::http::StatusCode,
        dioxus::server::axum::http::StatusCode,
    > {
        authorize_service(&headers)?;
        Uuid::parse_str(&id).map_err(|_| dioxus::server::axum::http::StatusCode::BAD_REQUEST)?;
        invalidate_access_token_cache()
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        access_token_secret_delete(&id)
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(dioxus::server::axum::http::StatusCode::NO_CONTENT)
    }

    pub async fn invalidate_access_token_cache_handler(
        headers: dioxus::server::axum::http::HeaderMap,
    ) -> std::result::Result<
        dioxus::server::axum::http::StatusCode,
        dioxus::server::axum::http::StatusCode,
    > {
        authorize_service(&headers)?;
        invalidate_access_token_cache()
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(dioxus::server::axum::http::StatusCode::NO_CONTENT)
    }

    pub async fn resolve_access_token_handler(
        dioxus::server::axum::extract::Path(access_key): dioxus::server::axum::extract::Path<
            String,
        >,
        headers: dioxus::server::axum::http::HeaderMap,
    ) -> std::result::Result<
        dioxus::server::axum::Json<ResolvedS3AccessToken>,
        dioxus::server::axum::http::StatusCode,
    > {
        authorize_service(&headers)?;
        let (generation, cached) = cached_access_token(&access_key)
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        if let Some(cached) = cached {
            return Ok(dioxus::server::axum::Json(cached));
        }
        let database = database()
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut rows = database
            .query_all(statement(
                "SELECT 'tenant'::text AS credential_kind, id, organization_id, project_id FROM storage_access_token WHERE access_key_id=$1 AND revoked_at IS NULL UNION ALL SELECT 'distribution'::text AS credential_kind, id, NULL::uuid AS organization_id, NULL::uuid AS project_id FROM registry_storage WHERE access_key_id=$1 AND service='distribution'",
                vec![access_key.clone().into()],
            ))
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        if rows.len() > 1 {
            return Err(dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
        let row = rows
            .pop()
            .ok_or(dioxus::server::axum::http::StatusCode::NOT_FOUND)?;
        let credential_kind: String = row
            .try_get("", "credential_kind")
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        let credential_id: Uuid = row
            .try_get("", "id")
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        let secret = match credential_kind.as_str() {
            "tenant" => access_token_secret_read(&credential_id.to_string()).await,
            "distribution" => service_credential_secret_read(&credential_id.to_string()).await,
            _ => return Err(dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        }
        .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        let organization_id: Option<Uuid> = row
            .try_get("", "organization_id")
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        let project_id: Option<Uuid> = row
            .try_get("", "project_id")
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        let bucket_rows = match credential_kind.as_str() {
            "tenant" => database
                .query_all(statement(
                    "SELECT permission.bucket_id, permission.can_read, permission.can_write, bucket.name AS bucket_name, region.slug AS region_slug, provider.id AS provider_id, CONCAT('cp-', REPLACE(bucket.id::text, '-', '')) AS physical_bucket_name FROM storage_access_token_bucket permission JOIN bucket ON bucket.id=permission.bucket_id JOIN regions region ON region.id=bucket.region JOIN s3_providers provider ON provider.id=region.s3_provider_id WHERE permission.access_token_id=$1 AND bucket.project_id=$2 AND provider.is_active=true",
                    vec![
                        credential_id.into(),
                        project_id
                            .ok_or(dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
                            .into(),
                    ],
                ))
                .await,
            "distribution" => database
                .query_all(statement(
                    "SELECT storage.id AS bucket_id, true AS can_read, true AS can_write, storage.bucket_name, provider.provider_region AS region_slug, provider.id AS provider_id, storage.physical_bucket_name FROM registry_storage storage JOIN s3_providers provider ON provider.id=storage.provider_id WHERE storage.id=$1 AND storage.service='distribution' AND provider.is_active=true",
                    vec![credential_id.into()],
                ))
                .await,
            _ => unreachable!(),
        }
        .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut bucket_permissions = Vec::with_capacity(bucket_rows.len());
        for bucket in bucket_rows {
            let bucket_id: Uuid = bucket
                .try_get("", "bucket_id")
                .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
            let provider_id: Uuid = bucket
                .try_get::<Uuid>("", "provider_id")
                .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
            bucket_permissions.push(ResolvedS3BucketPermission {
                bucket_id,
                bucket_name: bucket
                    .try_get("", "bucket_name")
                    .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?,
                physical_bucket_name: bucket
                    .try_get("", "physical_bucket_name")
                    .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?,
                region: bucket
                    .try_get("", "region_slug")
                    .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?,
                provider_id,
                platform_sse_key: bucket_key(bucket_id)
                    .await
                    .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?,
                can_read: bucket
                    .try_get("", "can_read")
                    .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?,
                can_write: bucket
                    .try_get("", "can_write")
                    .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?,
            });
        }
        let resolved = ResolvedS3AccessToken {
            organization_id,
            project_id,
            credential_id,
            bucket_permissions,
            secret_access_key: secret.secret_access_key,
        };
        cache_access_token(&access_key, generation, &resolved)
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(dioxus::server::axum::Json(resolved))
    }

    pub async fn eligible_regions_handler(
        headers: dioxus::server::axum::http::HeaderMap,
    ) -> std::result::Result<
        dioxus::server::axum::Json<Vec<EligibleRegion>>,
        dioxus::server::axum::http::StatusCode,
    > {
        authorize_service(&headers)?;
        let rows = database()
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
            .query_all(statement("SELECT id::text, slug, display_name, status::text, routing_mode::text FROM regions WHERE status='active' AND routing_mode <> 'disabled' ORDER BY display_name", vec![]))
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        let regions = rows
            .iter()
            .map(|row| {
                Ok(EligibleRegion {
                    id: text(row, "id")?,
                    slug: text(row, "slug")?,
                    display_name: text(row, "display_name")?,
                    status: text(row, "status")?,
                    routing_mode: text(row, "routing_mode")?,
                })
            })
            .collect::<Result<Vec<_>>>()
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(dioxus::server::axum::Json(regions))
    }

    pub async fn list_organizations() -> Result<Vec<Organization>> {
        let rows = database().await?.query_all(statement("SELECT o.id::text, o.name, o.email, o.slug, o.created_at::text, count(m.id) AS member_count FROM organization o LEFT JOIN organization_member m ON m.organization_id=o.id GROUP BY o.id ORDER BY o.name", vec![])).await.map_err(CapturedError::from_display)?;
        rows.iter()
            .map(|row| {
                Ok(Organization {
                    id: text(row, "id")?,
                    name: text(row, "name")?,
                    email: text(row, "email")?,
                    slug: text(row, "slug")?,
                    member_count: row
                        .try_get("", "member_count")
                        .map_err(CapturedError::from_display)?,
                    created_at: text(row, "created_at")?,
                })
            })
            .collect()
    }

    pub async fn update_organization(headers: &HeaderMap, id: String, name: String) -> Result<()> {
        let name = required(name, "name")?;
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        let result = tx
            .execute(statement(
                "UPDATE organization SET name=$2 WHERE id=$1::uuid",
                vec![id.clone().into(), name.clone().into()],
            ))
            .await
            .map_err(CapturedError::from_display)?;
        if result.rows_affected() == 0 {
            return Err(CapturedError::msg("Organization not found"));
        }
        audit(
            &tx,
            headers,
            "update",
            "organization",
            Some(&id),
            json!({"name": name}),
        )
        .await?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        Ok(())
    }

    pub async fn list_api_keys() -> Result<Vec<ApiKey>> {
        let rows = database().await?.query_all(statement("SELECT k.id::text, k.name, o.slug AS organization_slug, k.created_at::text FROM api_keys k JOIN organization o ON o.id=k.organization_id ORDER BY k.created_at DESC", vec![])).await.map_err(CapturedError::from_display)?;
        rows.iter()
            .map(|row| {
                Ok(ApiKey {
                    id: text(row, "id")?,
                    name: text(row, "name")?,
                    organization_slug: text(row, "organization_slug")?,
                    created_at: text(row, "created_at")?,
                })
            })
            .collect()
    }

    pub async fn list_audit_logs() -> Result<Vec<AuditLog>> {
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        tx.execute(statement("SET LOCAL ROLE app_audit_reader", vec![]))
            .await
            .map_err(CapturedError::from_display)?;
        let rows = tx
            .query_all(statement("SELECT id::text, actor_identifier, source_ip, action, resource_type, resource_id::text, changes::text, created_at::text FROM infrastructure_audit_log ORDER BY created_at DESC LIMIT 200", vec![]))
            .await
            .map_err(CapturedError::from_display)?;
        let logs = rows
            .iter()
            .map(|row| {
                Ok(AuditLog {
                    id: text(row, "id")?,
                    actor_identifier: text(row, "actor_identifier")?,
                    source_ip: text(row, "source_ip")?,
                    action: text(row, "action")?,
                    resource_type: text(row, "resource_type")?,
                    resource_id: optional_text(row, "resource_id")?,
                    changes: text(row, "changes")?,
                    created_at: text(row, "created_at")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        Ok(logs)
    }

    pub async fn delete_resource(
        headers: &HeaderMap,
        table: &str,
        resource_type: &str,
        id: String,
    ) -> Result<()> {
        let sql = match table {
            "regions" => "DELETE FROM regions WHERE id=$1::uuid",
            "clusters" => "DELETE FROM clusters WHERE id=$1::uuid",
            "organization" => "DELETE FROM organization WHERE id=$1::uuid",
            "api_keys" => "DELETE FROM api_keys WHERE id=$1::uuid",
            _ => return Err(CapturedError::msg("Invalid resource")),
        };
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        let result = tx
            .execute(statement(sql, vec![id.clone().into()]))
            .await
            .map_err(CapturedError::from_display)?;
        if result.rows_affected() == 0 {
            return Err(CapturedError::msg("Resource not found"));
        }
        audit(&tx, headers, "delete", resource_type, Some(&id), json!({})).await?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        if matches!(table, "regions" | "organization") {
            invalidate_access_token_cache().await?;
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::access_token_cache_key;

        #[test]
        fn access_token_cache_key_changes_with_generation() {
            assert_ne!(
                access_token_cache_key("CP123", 1),
                access_token_cache_key("CP123", 2)
            );
        }
    }
}
