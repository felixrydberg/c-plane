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
    name: String,
    endpoint_url: String,
    provider_region: String,
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
    is_active: bool,
) -> Result<()> {
    server::create_s3_provider(
        &headers,
        name,
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
    name: String,
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
        name,
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
    use chrono::{Duration, Utc};
    use dioxus::{CapturedError, fullstack::HeaderMap};
    use lib::secrets::{self, Client, PLATFORM_KEY};
    use sea_orm::{
        ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, QueryResult, Statement,
        TransactionTrait, Value,
    };
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::env;
    use tokio::sync::OnceCell;
    use uuid::Uuid;

    static DATABASE: OnceCell<DatabaseConnection> = OnceCell::const_new();
    static SECRETS: OnceCell<Client> = OnceCell::const_new();

    pub async fn initialize() -> Result<()> {
        required(
            env::var("CPLANE_SERVICE_TOKEN").map_err(CapturedError::from_display)?,
            "CPLANE_SERVICE_TOKEN",
        )?;
        database().await?;
        secrets().await?;
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
        let access_keys = access_keys_for_region(&id).await?;
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
        invalidate_access_token_caches(&access_keys).await?;
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
        let rows = database().await?.query_all(statement("SELECT id::text, name, endpoint_url, provider_region, is_active FROM s3_providers ORDER BY endpoint_url", vec![])).await.map_err(CapturedError::from_display)?;
        rows.iter()
            .map(|row| {
                Ok(S3Provider {
                    id: text(row, "id")?,
                    name: text(row, "name")?,
                    endpoint_url: text(row, "endpoint_url")?,
                    provider_region: optional_text(row, "provider_region")?,
                    is_active: row
                        .try_get("", "is_active")
                        .map_err(CapturedError::from_display)?,
                })
            })
            .collect()
    }

    async fn secrets() -> Result<&'static Client> {
        SECRETS
            .get_or_try_init(|| async { Client::from_env().map_err(CapturedError::from_display) })
            .await
    }

    async fn redis_connection() -> Result<redis::aio::MultiplexedConnection> {
        let url = env::var("REDIS_URL").map_err(CapturedError::from_display)?;
        let client = redis::Client::open(url).map_err(CapturedError::from_display)?;
        client
            .get_multiplexed_async_connection()
            .await
            .map_err(CapturedError::from_display)
    }

    async fn invalidate_access_token_caches(access_keys: &[String]) -> Result<()> {
        if access_keys.is_empty() {
            return Ok(());
        }
        let mut connection = redis_connection().await?;
        let _: u64 = redis::cmd("DEL")
            .arg(
                access_keys
                    .iter()
                    .map(|access_key| {
                        format!("{}{}", lib::cache::S3_ACCESS_TOKEN_CACHE_PREFIX, access_key)
                    })
                    .collect::<Vec<_>>(),
            )
            .query_async(&mut connection)
            .await
            .map_err(CapturedError::from_display)?;
        Ok(())
    }

    async fn invalidate_provider_credentials(provider_id: &str) -> Result<()> {
        let mut connection = redis_connection().await?;
        let _: u64 = redis::cmd("DEL")
            .arg(format!(
                "{}{}",
                lib::cache::S3_PROVIDER_CREDENTIAL_CACHE_PREFIX,
                provider_id
            ))
            .query_async(&mut connection)
            .await
            .map_err(CapturedError::from_display)?;
        Ok(())
    }

    async fn access_keys_for_region(region_id: &str) -> Result<Vec<String>> {
        let rows = database()
            .await?
            .query_all(statement(
                "SELECT DISTINCT token.access_key_id FROM storage_access_token token JOIN storage_access_token_bucket permission ON permission.access_token_id=token.id JOIN storage ON storage.id=permission.bucket_id WHERE storage.region_id=$1::uuid",
                vec![region_id.to_owned().into()],
            ))
            .await
            .map_err(CapturedError::from_display)?;
        rows.iter().map(|row| text(row, "access_key_id")).collect()
    }

    async fn access_keys_for_organization(organization_id: &str) -> Result<Vec<String>> {
        let rows = database()
            .await?
            .query_all(statement(
                "SELECT access_key_id FROM storage_access_token WHERE organization_id=$1::uuid",
                vec![organization_id.to_owned().into()],
            ))
            .await
            .map_err(CapturedError::from_display)?;
        rows.iter().map(|row| text(row, "access_key_id")).collect()
    }

    pub async fn create_s3_provider(
        headers: &HeaderMap,
        name: String,
        endpoint_url: String,
        provider_region: String,
        credentials: S3Credentials,
        is_active: bool,
    ) -> Result<()> {
        let id = Uuid::new_v4();
        let name = required(name, "name")?;
        let endpoint_url = required(endpoint_url, "endpoint URL")?;
        let provider_region = required(provider_region, "provider region")?;
        required(credentials.access_key_id.clone(), "access key ID")?;
        required(credentials.secret_access_key.clone(), "secret access key")?;
        let ciphertext = secrets::encrypt(
            secrets().await?,
            PLATFORM_KEY,
            &serde_json::to_vec(&credentials).map_err(CapturedError::from_display)?,
        )
        .await
        .map_err(CapturedError::from_display)?;
        let credential_secret_id = Uuid::new_v4();
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        tx.execute(statement(
            "INSERT INTO secret (id, scope, organization_id, ciphertext) VALUES ($1::uuid, 'platform'::secret_scope, NULL, $2)",
            vec![credential_secret_id.into(), ciphertext.into()],
        ))
        .await
        .map_err(CapturedError::from_display)?;
        tx.execute(statement("INSERT INTO s3_providers (id, name, endpoint_url, provider_region, credential_secret_id, is_active) VALUES ($1::uuid, $2, $3, $4, $5::uuid, $6)", vec![id.into(), name.clone().into(), endpoint_url.clone().into(), provider_region.clone().into(), credential_secret_id.into(), is_active.into()])).await.map_err(CapturedError::from_display)?;
        audit(&tx, headers, "create", "s3_provider", Some(&id.to_string()), json!({"name": name, "endpoint_url": endpoint_url, "provider_region": provider_region, "is_active": is_active})).await?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        invalidate_provider_credentials(&id.to_string()).await
    }

    pub async fn update_s3_provider(
        headers: &HeaderMap,
        id: String,
        name: String,
        endpoint_url: String,
        provider_region: String,
        credentials: Option<S3Credentials>,
        is_active: bool,
    ) -> Result<()> {
        let name = required(name, "name")?;
        let endpoint_url = required(endpoint_url, "endpoint URL")?;
        let provider_region = required(provider_region, "provider region")?;
        let replacement_ciphertext = match credentials.as_ref() {
            Some(replacement) => Some(
                secrets::encrypt(
                    secrets().await?,
                    PLATFORM_KEY,
                    &serde_json::to_vec(replacement).map_err(CapturedError::from_display)?,
                )
                .await
                .map_err(CapturedError::from_display)?,
            ),
            None => None,
        };
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        let row = tx
            .query_one(statement(
                "SELECT credential_secret_id FROM s3_providers WHERE id=$1::uuid FOR UPDATE",
                vec![id.clone().into()],
            ))
            .await
            .map_err(CapturedError::from_display)?
            .ok_or_else(|| CapturedError::msg("S3 provider not found"))?;
        let credential_secret_id: Uuid = row
            .try_get("", "credential_secret_id")
            .map_err(CapturedError::from_display)?;
        if let Some(ciphertext) = replacement_ciphertext {
            let updated = tx
                .execute(statement(
                    "UPDATE secret SET ciphertext=$2, updated_at=NOW() WHERE id=$1::uuid",
                    vec![credential_secret_id.into(), ciphertext.into()],
                ))
                .await
                .map_err(CapturedError::from_display)?;
            if updated.rows_affected() != 1 {
                return Err(CapturedError::msg("S3 provider secret not found"));
            }
        }
        tx.execute(statement("UPDATE s3_providers SET name=$2, endpoint_url=$3, provider_region=$4, is_active=$5, updated_at=now() WHERE id=$1::uuid", vec![id.clone().into(), name.clone().into(), endpoint_url.clone().into(), provider_region.clone().into(), is_active.into()])).await.map_err(CapturedError::from_display)?;
        audit(&tx, headers, "update", "s3_provider", Some(&id), json!({"name": name, "endpoint_url": endpoint_url, "provider_region": provider_region, "is_active": is_active, "credentials_rotated": credentials.is_some()})).await?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        invalidate_provider_credentials(&id).await
    }

    pub async fn delete_s3_provider(headers: &HeaderMap, id: String) -> Result<()> {
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        let row = tx
            .query_one(statement(
                "SELECT credential_secret_id FROM s3_providers WHERE id=$1::uuid FOR UPDATE",
                vec![id.clone().into()],
            ))
            .await
            .map_err(CapturedError::from_display)?
            .ok_or_else(|| CapturedError::msg("S3 provider not found"))?;
        let credential_secret_id: Uuid = row
            .try_get("", "credential_secret_id")
            .map_err(CapturedError::from_display)?;
        tx.execute(statement(
            "DELETE FROM s3_providers WHERE id=$1::uuid",
            vec![id.clone().into()],
        ))
        .await
        .map_err(CapturedError::from_display)?;
        tx.execute(statement(
            "DELETE FROM secret WHERE id=$1::uuid",
            vec![credential_secret_id.into()],
        ))
        .await
        .map_err(CapturedError::from_display)?;
        audit(&tx, headers, "delete", "s3_provider", Some(&id), json!({})).await?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        invalidate_provider_credentials(&id).await
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
        let access_keys = match table {
            "regions" => access_keys_for_region(&id).await?,
            "organization" => access_keys_for_organization(&id).await?,
            _ => Vec::new(),
        };
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
            invalidate_access_token_caches(&access_keys).await?;
        }
        Ok(())
    }
}
