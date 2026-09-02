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
    clickhouse_provider_id: Option<String>,
) -> Result<()> {
    server::create_region(
        &headers,
        slug,
        display_name,
        status,
        s3_provider_id,
        clickhouse_provider_id,
    )
    .await
}

#[patch("/api/infrastructure/regions/{id}", headers: dioxus::fullstack::HeaderMap)]
pub async fn update_region(
    id: String,
    slug: String,
    display_name: String,
    status: String,
    s3_provider_id: Option<String>,
    clickhouse_provider_id: Option<String>,
) -> Result<()> {
    server::update_region(
        &headers,
        id,
        slug,
        display_name,
        status,
        s3_provider_id,
        clickhouse_provider_id,
    )
    .await
}

#[delete("/api/infrastructure/regions/{id}", headers: dioxus::fullstack::HeaderMap)]
pub async fn delete_region(id: String) -> Result<()> {
    server::delete_region(&headers, id).await
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

#[get("/api/infrastructure/clickhouse-providers")]
pub async fn list_clickhouse_providers() -> Result<Vec<ClickHouseProvider>> {
    server::list_clickhouse_providers().await
}

#[post("/api/infrastructure/clickhouse-providers", headers: dioxus::fullstack::HeaderMap)]
pub async fn create_clickhouse_provider(
    name: String,
    endpoint_url: String,
    cluster_name: String,
    username: String,
    password: String,
    s3_provider_id: String,
) -> Result<CreatedClickHouseProvider> {
    server::create_clickhouse_provider(
        &headers,
        name,
        endpoint_url,
        cluster_name,
        ClickHouseCredentials { username, password },
        s3_provider_id,
        None,
    )
    .await
}

#[patch("/api/infrastructure/clickhouse-providers/{id}", headers: dioxus::fullstack::HeaderMap)]
pub async fn update_clickhouse_provider(
    id: String,
    name: String,
    endpoint_url: String,
    cluster_name: String,
    username: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let credentials = match (username, password) {
        (Some(username), Some(password)) => Some(ClickHouseCredentials { username, password }),
        (None, None) => None,
        _ => {
            return Err(dioxus::CapturedError::msg(
                "Username and password must be replaced together",
            ));
        }
    };
    server::update_clickhouse_provider(&headers, id, name, endpoint_url, cluster_name, credentials)
        .await
}

#[delete("/api/infrastructure/clickhouse-providers/{id}", headers: dioxus::fullstack::HeaderMap)]
pub async fn delete_clickhouse_provider(id: String) -> Result<()> {
    server::delete_clickhouse_provider(&headers, id).await
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
    use lib::{
        buckets,
        entities::{
            bucket, bucket_grant, clickhouse_provider, credential, region, s3_provider, secret,
            storage_access_token,
        },
        provisioning::provision_platform_bucket,
        secrets::{self, Client, PLATFORM_KEY},
    };
    use sea_orm::{
        ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseBackend,
        DatabaseConnection, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder,
        QueryResult, Set, Statement, TransactionTrait, Value,
    };
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::env;
    use tokio::sync::OnceCell;
    use uuid::Uuid;

    static DATABASE: OnceCell<DatabaseConnection> = OnceCell::const_new();
    static SECRETS: OnceCell<Client> = OnceCell::const_new();
    const CLICKHOUSE_BUCKET_NAME: &str = "clickhouse";

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

    fn region_status(value: &str) -> region::RegionStatus {
        match value {
            "active" => region::RegionStatus::Active,
            "inactive" => region::RegionStatus::Inactive,
            "maintenance" => region::RegionStatus::Maintenance,
            _ => unreachable!("region status was validated before conversion"),
        }
    }

    fn region_status_name(value: &region::RegionStatus) -> &'static str {
        match value {
            region::RegionStatus::Active => "active",
            region::RegionStatus::Inactive => "inactive",
            region::RegionStatus::Maintenance => "maintenance",
        }
    }

    fn region_routing_mode_name(value: &region::RegionRoutingMode) -> &'static str {
        match value {
            region::RegionRoutingMode::Active => "active",
            region::RegionRoutingMode::Draining => "draining",
            region::RegionRoutingMode::Disabled => "disabled",
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
        let rows = region::Entity::find()
            .find_also_related(clickhouse_provider::Entity)
            .order_by_asc(region::Column::DisplayName)
            .all(database().await?)
            .await
            .map_err(CapturedError::from_display)?;
        rows.into_iter()
            .map(|(region, clickhouse)| {
                let clickhouse = clickhouse;
                Ok(Region {
                    id: region.id.to_string(),
                    slug: region.slug,
                    display_name: region.display_name,
                    status: region_status_name(&region.status).to_string(),
                    s3_provider_id: region.s3_provider_id.map(|id| id.to_string()),
                    clickhouse_provider_id: region.clickhouse_provider_id.map(|id| id.to_string()),
                    clickhouse_provider_name: clickhouse.map(|provider| provider.name),
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
        clickhouse_provider_id: Option<String>,
    ) -> Result<()> {
        let id = Uuid::new_v4();
        let slug = validate_region_slug(slug)?;
        let display_name = required(display_name, "display name")?;
        let status = validate_choice(status, &["active", "inactive", "maintenance"], "status")?;
        let s3_provider_id = s3_provider_id
            .map(|id| Uuid::parse_str(&id).map_err(CapturedError::from_display))
            .transpose()?;
        let clickhouse_provider_id = clickhouse_provider_id
            .map(|id| Uuid::parse_str(&id).map_err(CapturedError::from_display))
            .transpose()?;
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        region::ActiveModel {
            id: Set(id),
            slug: Set(slug.clone()),
            display_name: Set(display_name.clone()),
            status: Set(region_status(&status)),
            s3_provider_id: Set(s3_provider_id),
            clickhouse_provider_id: Set(clickhouse_provider_id),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(CapturedError::from_display)?;
        audit(&tx, headers, "create", "region", Some(&id.to_string()), json!({"slug": slug, "display_name": display_name, "status": status, "s3_provider_id": s3_provider_id, "clickhouse_provider_id": clickhouse_provider_id})).await?;
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
        clickhouse_provider_id: Option<String>,
    ) -> Result<()> {
        let slug = validate_region_slug(slug)?;
        let display_name = required(display_name, "display name")?;
        let status = validate_choice(status, &["active", "inactive", "maintenance"], "status")?;
        let id = Uuid::parse_str(&id).map_err(CapturedError::from_display)?;
        let s3_provider_id = s3_provider_id
            .map(|id| Uuid::parse_str(&id).map_err(CapturedError::from_display))
            .transpose()?;
        let clickhouse_provider_id = clickhouse_provider_id
            .map(|id| Uuid::parse_str(&id).map_err(CapturedError::from_display))
            .transpose()?;
        let access_keys = access_keys_for_region(&id.to_string()).await?;
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        let mut active = region::Entity::find_by_id(id)
            .one(&tx)
            .await
            .map_err(CapturedError::from_display)?
            .ok_or_else(|| CapturedError::msg("Region not found"))?
            .into_active_model();
        active.slug = Set(slug.clone());
        active.display_name = Set(display_name.clone());
        active.status = Set(region_status(&status));
        active.s3_provider_id = Set(s3_provider_id);
        active.clickhouse_provider_id = Set(clickhouse_provider_id);
        active.updated_at = Set(Utc::now().fixed_offset());
        active
            .update(&tx)
            .await
            .map_err(CapturedError::from_display)?;
        audit(&tx, headers, "update", "region", Some(&id.to_string()), json!({"slug": slug, "display_name": display_name, "status": status, "s3_provider_id": s3_provider_id, "clickhouse_provider_id": clickhouse_provider_id})).await?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        invalidate_access_token_caches(&access_keys).await?;
        Ok(())
    }

    pub async fn delete_region(headers: &HeaderMap, id: String) -> Result<()> {
        let access_keys = access_keys_for_region(&id).await?;
        let id = Uuid::parse_str(&id).map_err(CapturedError::from_display)?;
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        let deleted = region::Entity::delete_by_id(id)
            .exec(&tx)
            .await
            .map_err(CapturedError::from_display)?;
        if deleted.rows_affected == 0 {
            return Err(CapturedError::msg("Resource not found"));
        }
        audit(
            &tx,
            headers,
            "delete",
            "region",
            Some(&id.to_string()),
            json!({}),
        )
        .await?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        invalidate_access_token_caches(&access_keys).await
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
        let providers = s3_provider::Entity::find()
            .order_by_asc(s3_provider::Column::EndpointUrl)
            .all(database().await?)
            .await
            .map_err(CapturedError::from_display)?;
        Ok(providers
            .into_iter()
            .map(|provider| S3Provider {
                id: provider.id.to_string(),
                name: provider.name,
                endpoint_url: provider.endpoint_url,
                provider_region: Some(provider.provider_region),
                is_active: provider.is_active,
            })
            .collect())
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
        let organization_id =
            Uuid::parse_str(organization_id).map_err(CapturedError::from_display)?;
        let rows = storage_access_token::Entity::find()
            .filter(storage_access_token::Column::OrganizationId.eq(organization_id))
            .find_also_related(credential::Entity)
            .all(database().await?)
            .await
            .map_err(CapturedError::from_display)?;
        rows.into_iter()
            .map(|(_, credential)| {
                credential
                    .map(|credential| credential.access_key_id)
                    .ok_or_else(|| CapturedError::msg("Storage credential not found"))
            })
            .collect()
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
        secret::ActiveModel {
            id: Set(credential_secret_id),
            scope: Set(secret::SecretScope::Platform),
            organization_id: Set(None),
            ciphertext: Set(ciphertext),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(CapturedError::from_display)?;
        s3_provider::ActiveModel {
            id: Set(id),
            name: Set(name.clone()),
            endpoint_url: Set(endpoint_url.clone()),
            provider_region: Set(provider_region.clone()),
            credential_secret_id: Set(credential_secret_id),
            is_active: Set(is_active),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(CapturedError::from_display)?;
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
        let id_uuid = Uuid::parse_str(&id).map_err(CapturedError::from_display)?;
        let provider = s3_provider::Entity::find_by_id(id_uuid)
            .one(&tx)
            .await
            .map_err(CapturedError::from_display)?
            .ok_or_else(|| CapturedError::msg("S3 provider not found"))?;
        let credential_secret_id = provider.credential_secret_id;
        if let Some(ciphertext) = replacement_ciphertext {
            let mut secret = secret::Entity::find_by_id(credential_secret_id)
                .one(&tx)
                .await
                .map_err(CapturedError::from_display)?
                .ok_or_else(|| CapturedError::msg("S3 provider secret not found"))?
                .into_active_model();
            secret.ciphertext = Set(ciphertext);
            secret.updated_at = Set(Utc::now().fixed_offset());
            secret
                .update(&tx)
                .await
                .map_err(CapturedError::from_display)?;
        }
        let mut provider = provider.into_active_model();
        provider.name = Set(name.clone());
        provider.endpoint_url = Set(endpoint_url.clone());
        provider.provider_region = Set(provider_region.clone());
        provider.is_active = Set(is_active);
        provider.updated_at = Set(Utc::now().fixed_offset());
        provider
            .update(&tx)
            .await
            .map_err(CapturedError::from_display)?;
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
        let id_uuid = Uuid::parse_str(&id).map_err(CapturedError::from_display)?;
        let provider = s3_provider::Entity::find_by_id(id_uuid)
            .one(&tx)
            .await
            .map_err(CapturedError::from_display)?
            .ok_or_else(|| CapturedError::msg("S3 provider not found"))?;
        s3_provider::Entity::delete_by_id(provider.id)
            .exec(&tx)
            .await
            .map_err(CapturedError::from_display)?;
        secret::Entity::delete_by_id(provider.credential_secret_id)
            .exec(&tx)
            .await
            .map_err(CapturedError::from_display)?;
        audit(&tx, headers, "delete", "s3_provider", Some(&id), json!({})).await?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        invalidate_provider_credentials(&id).await
    }

    async fn insert_clickhouse_provider(
        connection: &impl ConnectionTrait,
        id: Uuid,
        name: &str,
        endpoint_url: &str,
        cluster_name: &str,
        credentials: &ClickHouseCredentials,
        storage: &lib::provisioning::ProvisionedPlatformBucket,
    ) -> Result<()> {
        let credential_secret_id = Uuid::new_v4();
        let ciphertext = secrets::encrypt(
            secrets().await?,
            PLATFORM_KEY,
            &serde_json::to_vec(credentials).map_err(CapturedError::from_display)?,
        )
        .await
        .map_err(CapturedError::from_display)?;
        secret::ActiveModel {
            id: Set(credential_secret_id),
            scope: Set(secret::SecretScope::Platform),
            organization_id: Set(None),
            ciphertext: Set(ciphertext),
            ..Default::default()
        }
        .insert(connection)
        .await
        .map_err(CapturedError::from_display)?;
        clickhouse_provider::ActiveModel {
            id: Set(id),
            name: Set(name.to_owned()),
            endpoint_url: Set(endpoint_url.to_owned()),
            cluster_name: Set(cluster_name.to_owned()),
            credential_secret_id: Set(credential_secret_id),
            bucket_id: Set(storage.bucket_id),
            storage_credential_id: Set(storage.storage_credential.id),
            ..Default::default()
        }
        .insert(connection)
        .await
        .map_err(CapturedError::from_display)?;
        Ok(())
    }

    pub async fn list_clickhouse_providers() -> Result<Vec<ClickHouseProvider>> {
        let rows = clickhouse_provider::Entity::find()
            .order_by_asc(clickhouse_provider::Column::Name)
            .all(database().await?)
            .await
            .map_err(CapturedError::from_display)?;
        let mut providers = Vec::with_capacity(rows.len());
        for provider in rows {
            let bucket = bucket::Entity::find_by_id(provider.bucket_id)
                .one(database().await?)
                .await
                .map_err(CapturedError::from_display)?
                .ok_or_else(|| CapturedError::msg("ClickHouse bucket not found"))?;
            let bucket_provider = s3_provider::Entity::find_by_id(bucket.s3_provider_id)
                .one(database().await?)
                .await
                .map_err(CapturedError::from_display)?
                .ok_or_else(|| CapturedError::msg("ClickHouse bucket S3 provider not found"))?;
            let storage_credential = credential::Entity::find_by_id(provider.storage_credential_id)
                .one(database().await?)
                .await
                .map_err(CapturedError::from_display)?
                .ok_or_else(|| CapturedError::msg("ClickHouse storage credential not found"))?;
            providers.push(ClickHouseProvider {
                id: provider.id.to_string(),
                name: provider.name,
                endpoint_url: provider.endpoint_url,
                cluster_name: provider.cluster_name,
                bucket_id: provider.bucket_id.to_string(),
                s3_provider_id: bucket.s3_provider_id.to_string(),
                s3_provider_name: bucket_provider.name,
                storage_access_key_id: storage_credential.access_key_id,
                created_at: provider.created_at.to_rfc3339(),
                updated_at: provider.updated_at.to_rfc3339(),
            });
        }
        Ok(providers)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_clickhouse_provider(
        headers: &HeaderMap,
        name: String,
        endpoint_url: String,
        cluster_name: String,
        credentials: ClickHouseCredentials,
        s3_provider_id: String,
        storage_credentials: Option<(String, String)>,
    ) -> Result<CreatedClickHouseProvider> {
        let id = Uuid::new_v4();
        let name = required(name, "name")?;
        let endpoint_url = required(endpoint_url, "endpoint URL")?;
        let cluster_name = required(cluster_name, "cluster name")?;
        required(credentials.username.clone(), "username")?;
        required(credentials.password.clone(), "password")?;
        let s3_provider_id = Uuid::parse_str(&required(s3_provider_id, "S3 provider")?)
            .map_err(CapturedError::from_display)?;
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        let storage =
            provision_platform_bucket(&tx, secrets().await?, s3_provider_id, storage_credentials)
                .await
                .map_err(CapturedError::from_display)?;
        let result = async {
            insert_clickhouse_provider(&tx, id, &name, &endpoint_url, &cluster_name, &credentials, &storage).await?;
            audit(&tx, headers, "create", "clickhouse_provider", Some(&id.to_string()), json!({"name": name, "endpoint_url": endpoint_url, "cluster_name": cluster_name, "s3_provider_id": s3_provider_id})).await?;
            tx.commit().await.map_err(CapturedError::from_display)
        }.await;
        if let Err(error) = result {
            let _ = buckets::delete(&storage.provider, storage.bucket_id).await;
            return Err(error);
        }
        Ok(CreatedClickHouseProvider {
            provider: ClickHouseProvider {
                id: id.to_string(),
                name,
                endpoint_url,
                cluster_name,
                bucket_id: storage.bucket_id.to_string(),
                s3_provider_id: s3_provider_id.to_string(),
                s3_provider_name: String::new(),
                storage_access_key_id: storage.access_key_id,
                created_at: String::new(),
                updated_at: String::new(),
            },
            storage_endpoint_url: env::var("STORAGE_INTERNAL_URL")
                .unwrap_or_else(|_| "http://storage:8081".to_string()),
            bucket_name: CLICKHOUSE_BUCKET_NAME.to_string(),
            secret_access_key: storage.secret_access_key,
        })
    }

    pub async fn update_clickhouse_provider(
        headers: &HeaderMap,
        id: String,
        name: String,
        endpoint_url: String,
        cluster_name: String,
        credentials: Option<ClickHouseCredentials>,
    ) -> Result<()> {
        let name = required(name, "name")?;
        let endpoint_url = required(endpoint_url, "endpoint URL")?;
        let cluster_name = required(cluster_name, "cluster name")?;
        let replacement_ciphertext = match credentials.as_ref() {
            Some(credentials) => {
                required(credentials.username.clone(), "username")?;
                required(credentials.password.clone(), "password")?;
                Some(
                    secrets::encrypt(
                        secrets().await?,
                        PLATFORM_KEY,
                        &serde_json::to_vec(credentials).map_err(CapturedError::from_display)?,
                    )
                    .await
                    .map_err(CapturedError::from_display)?,
                )
            }
            None => None,
        };
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        let id_uuid = Uuid::parse_str(&id).map_err(CapturedError::from_display)?;
        let provider = clickhouse_provider::Entity::find_by_id(id_uuid)
            .one(&tx)
            .await
            .map_err(CapturedError::from_display)?
            .ok_or_else(|| CapturedError::msg("ClickHouse provider not found"))?;
        let credential_secret_id = provider.credential_secret_id;
        if let Some(ciphertext) = replacement_ciphertext {
            let mut secret = secret::Entity::find_by_id(credential_secret_id)
                .one(&tx)
                .await
                .map_err(CapturedError::from_display)?
                .ok_or_else(|| CapturedError::msg("ClickHouse provider secret not found"))?
                .into_active_model();
            secret.ciphertext = Set(ciphertext);
            secret.updated_at = Set(Utc::now().fixed_offset());
            secret
                .update(&tx)
                .await
                .map_err(CapturedError::from_display)?;
        }
        let mut provider = provider.into_active_model();
        provider.name = Set(name.clone());
        provider.endpoint_url = Set(endpoint_url.clone());
        provider.cluster_name = Set(cluster_name.clone());
        provider.updated_at = Set(Utc::now().fixed_offset());
        provider
            .update(&tx)
            .await
            .map_err(CapturedError::from_display)?;
        audit(&tx, headers, "update", "clickhouse_provider", Some(&id), json!({"name": name, "endpoint_url": endpoint_url, "cluster_name": cluster_name, "credentials_rotated": credentials.is_some()})).await?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        Ok(())
    }

    pub async fn delete_clickhouse_provider(headers: &HeaderMap, id: String) -> Result<()> {
        let tx = database()
            .await?
            .begin()
            .await
            .map_err(CapturedError::from_display)?;
        let id_uuid = Uuid::parse_str(&id).map_err(CapturedError::from_display)?;
        let referenced = region::Entity::find()
            .filter(region::Column::ClickhouseProviderId.eq(id_uuid))
            .count(&tx)
            .await
            .map_err(CapturedError::from_display)?;
        if referenced != 0 {
            return Err(CapturedError::msg(
                "ClickHouse provider is still assigned to one or more regions",
            ));
        }
        let provider = clickhouse_provider::Entity::find_by_id(id_uuid)
            .one(&tx)
            .await
            .map_err(CapturedError::from_display)?
            .ok_or_else(|| CapturedError::msg("ClickHouse provider not found"))?;
        let bucket = bucket::Entity::find_by_id(provider.bucket_id)
            .one(&tx)
            .await
            .map_err(CapturedError::from_display)?
            .ok_or_else(|| CapturedError::msg("ClickHouse bucket not found"))?;
        let credential = credential::Entity::find_by_id(provider.storage_credential_id)
            .one(&tx)
            .await
            .map_err(CapturedError::from_display)?
            .ok_or_else(|| CapturedError::msg("ClickHouse storage credential not found"))?;
        let access_key_id = credential.access_key_id;
        clickhouse_provider::Entity::delete_by_id(provider.id)
            .exec(&tx)
            .await
            .map_err(CapturedError::from_display)?;
        bucket_grant::Entity::delete_many()
            .filter(bucket_grant::Column::CredentialId.eq(provider.storage_credential_id))
            .filter(bucket_grant::Column::BucketId.eq(provider.bucket_id))
            .exec(&tx)
            .await
            .map_err(CapturedError::from_display)?;
        buckets::credentials::delete(&tx, provider.storage_credential_id)
            .await
            .map_err(CapturedError::from_display)?;
        bucket::Entity::delete_by_id(bucket.id)
            .exec(&tx)
            .await
            .map_err(CapturedError::from_display)?;
        secret::Entity::delete_many()
            .filter(secret::Column::Id.is_in([provider.credential_secret_id, bucket.sse_secret_id]))
            .exec(&tx)
            .await
            .map_err(CapturedError::from_display)?;
        audit(
            &tx,
            headers,
            "delete",
            "clickhouse_provider",
            Some(&id),
            json!({"bucket_id": bucket.id}),
        )
        .await?;
        tx.commit().await.map_err(CapturedError::from_display)?;
        invalidate_access_token_caches(&[access_key_id]).await
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
        let rows = region::Entity::find()
            .filter(region::Column::Status.eq(region::RegionStatus::Active))
            .filter(region::Column::RoutingMode.ne(region::RegionRoutingMode::Disabled))
            .order_by_asc(region::Column::DisplayName)
            .all(
                database()
                    .await
                    .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?,
            )
            .await
            .map_err(|_| dioxus::server::axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
        let regions = rows
            .into_iter()
            .map(|region| EligibleRegion {
                id: region.id.to_string(),
                slug: region.slug,
                display_name: region.display_name,
                status: region_status_name(&region.status).to_string(),
                routing_mode: region_routing_mode_name(&region.routing_mode).to_string(),
            })
            .collect();
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
