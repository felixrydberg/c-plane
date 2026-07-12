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
    provider_region: Option<String>,
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
    provider_region: Option<String>,
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
    use chrono::{Duration, Utc};
    use dioxus::{CapturedError, fullstack::HeaderMap};
    use reqwest::Client;
    use sea_orm::{
        ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, QueryResult, Statement,
        TransactionTrait, Value,
    };
    use serde::Deserialize;
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::{collections::HashMap, env};
    use tokio::sync::{OnceCell, RwLock};
    use uuid::Uuid;

    static DATABASE: OnceCell<DatabaseConnection> = OnceCell::const_new();
    static CREDENTIALS: OnceCell<RwLock<HashMap<String, S3Credentials>>> = OnceCell::const_new();

    pub async fn initialize() -> Result<()> {
        required(
            env::var("CPLANE_SERVICE_TOKEN").map_err(CapturedError::from_display)?,
            "CPLANE_SERVICE_TOKEN",
        )?;
        database().await?;
        credential_cache().await?;
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

    async fn credential_cache() -> Result<&'static RwLock<HashMap<String, S3Credentials>>> {
        CREDENTIALS
            .get_or_try_init(|| async {
                let rows = database()
                    .await?
                    .query_all(statement("SELECT id::text FROM s3_providers", vec![]))
                    .await
                    .map_err(CapturedError::from_display)?;
                let mut credentials = HashMap::new();
                for row in rows {
                    let id = text(&row, "id")?;
                    credentials.insert(id.clone(), openbao_read(&id).await?);
                }
                Ok(RwLock::new(credentials))
            })
            .await
    }

    pub async fn create_s3_provider(
        headers: &HeaderMap,
        provider_type: String,
        endpoint_url: String,
        provider_region: Option<String>,
        credentials: S3Credentials,
        is_active: bool,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let provider_type =
            validate_choice(provider_type, &["aws_s3", "cloudflare_r2"], "provider type")?;
        let endpoint_url = required(endpoint_url, "endpoint URL")?;
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
        credential_cache()
            .await?
            .write()
            .await
            .insert(id, credentials);
        Ok(())
    }

    pub async fn update_s3_provider(
        headers: &HeaderMap,
        id: String,
        provider_type: String,
        endpoint_url: String,
        provider_region: Option<String>,
        credentials: Option<S3Credentials>,
        is_active: bool,
    ) -> Result<()> {
        let provider_type =
            validate_choice(provider_type, &["aws_s3", "cloudflare_r2"], "provider type")?;
        let endpoint_url = required(endpoint_url, "endpoint URL")?;
        let previous_credentials = if credentials.is_some() {
            credential_cache().await?.read().await.get(&id).cloned()
        } else {
            None
        };
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
            if let Some(previous) = previous_credentials {
                let _ = openbao_write(&id, &previous).await;
            }
            return Err(error);
        }
        if let Some(credentials) = credentials {
            credential_cache()
                .await?
                .write()
                .await
                .insert(id, credentials);
        }
        Ok(())
    }

    pub async fn delete_s3_provider(headers: &HeaderMap, id: String) -> Result<()> {
        let old = credential_cache()
            .await?
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or_else(|| CapturedError::msg("S3 provider credentials not found"))?;
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
        credential_cache().await?.write().await.remove(&id);
        Ok(())
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
        dioxus::server::axum::Json<S3Credentials>,
        dioxus::server::axum::http::StatusCode,
    > {
        authorize_service(&headers)?;
        credential_cache()
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
            .read()
            .await
            .get(&id)
            .cloned()
            .map(dioxus::server::axum::Json)
            .ok_or(dioxus::server::axum::http::StatusCode::NOT_FOUND)
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
        Ok(())
    }
}
