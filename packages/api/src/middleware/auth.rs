use axum::extract::{ConnectInfo, FromRequestParts, MatchedPath};
use axum::http::request::Parts;
use reqwest::Client;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use std::{collections::HashSet, sync::OnceLock};
use uuid::Uuid;

use crate::errors::AppError;
use crate::state::{AppDatabase, OrganizationContext, TenantDatabase, get_app_state};

fn reqwest_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client")
    })
}

#[derive(Clone, Debug)]
pub struct RequestAuthContext {
    pub actor_id: Uuid,
    api_key_scopes: Option<HashSet<String>>,
}

impl RequestAuthContext {
    pub fn require_scope(&self, scope: &str) -> Result<(), AppError> {
        if self
            .api_key_scopes
            .as_ref()
            .is_none_or(|scopes| scopes.contains(scope))
        {
            return Ok(());
        }

        Err(AppError::Forbidden(format!(
            "API key is missing required scope: {scope}"
        )))
    }
}

pub(crate) fn required_scope(method: &str, path: &str) -> Option<&'static str> {
    Some(match (method, path) {
        ("GET", "/api/organization/{organization_id}/regions") => "region:read",
        ("GET", "/api/organization/{organization_id}/projects") => "project:read",
        ("POST", "/api/organization/{organization_id}/projects") => "project:create",
        ("GET", "/api/organization/{organization_id}/projects/{project_id}") => "project:read",
        ("DELETE", "/api/organization/{organization_id}/projects/{project_id}") => "project:delete",
        ("GET", "/api/organization/{organization_id}/environments") => "project:read",
        ("GET", "/api/organization/{organization_id}/projects/{project_id}/environments") => {
            "project:read"
        }
        ("POST", "/api/organization/{organization_id}/projects/{project_id}/environments") => {
            "project:manage"
        }
        (
            "PATCH",
            "/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}",
        ) => "project:manage",
        (
            "DELETE",
            "/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}",
        ) => "project:manage",
        ("GET", "/api/organization/{organization_id}/projects/{project_id}/timelines") => {
            "timeline:read"
        }
        (
            "GET",
            "/api/organization/{organization_id}/projects/{project_id}/timelines/{timeline_id}",
        ) => "timeline:read",
        ("GET", "/api/organization/{organization_id}/events") => "event:read",
        ("GET", "/api/organization/{organization_id}/containers") => "container:read",
        ("POST", "/api/organization/{organization_id}/containers") => "container:create",
        ("GET", "/api/organization/{organization_id}/containers/{container_id}") => {
            "container:read"
        }
        ("PATCH", "/api/organization/{organization_id}/containers/{container_id}") => {
            "container:update"
        }
        ("POST", "/api/organization/{organization_id}/containers/{container_id}/deploy") => {
            "container:update"
        }
        ("DELETE", "/api/organization/{organization_id}/containers/{container_id}") => {
            "container:delete"
        }
        ("GET", "/api/organization/{organization_id}/databases/postgres") => {
            "database:postgres:read"
        }
        ("POST", "/api/organization/{organization_id}/databases/postgres") => {
            "database:postgres:create"
        }
        ("GET", "/api/organization/{organization_id}/databases/postgres/{database_id}") => {
            "database:postgres:read"
        }
        ("PATCH", "/api/organization/{organization_id}/databases/postgres/{database_id}") => {
            "database:postgres:update"
        }
        ("DELETE", "/api/organization/{organization_id}/databases/postgres/{database_id}") => {
            "database:postgres:delete"
        }
        (
            "GET",
            "/api/organization/{organization_id}/databases/postgres/{database_id}/branches",
        ) => "database:postgres:read",
        (
            "POST",
            "/api/organization/{organization_id}/databases/postgres/{database_id}/branches",
        ) => "database:postgres:manage",
        (
            "PATCH",
            "/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}",
        ) => "database:postgres:manage",
        (
            "DELETE",
            "/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}",
        ) => "database:postgres:manage",
        (
            "GET",
            "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens",
        ) => "access-token:read",
        (
            "POST",
            "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens",
        ) => "access-token:create",
        (
            "GET",
            "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
        ) => "access-token:read",
        (
            "PATCH",
            "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
        ) => "access-token:update",
        (
            "DELETE",
            "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
        ) => "access-token:delete",
        ("GET", "/api/organization/{organization_id}/storage/buckets") => "bucket:read",
        ("POST", "/api/organization/{organization_id}/storage/buckets") => "bucket:create",
        ("DELETE", "/api/organization/{organization_id}/storage/buckets/{bucket_id}") => {
            "bucket:delete"
        }
        ("GET", "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects") => {
            "bucket:read"
        }
        ("DELETE", "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects") => {
            "bucket:delete"
        }
        (
            "GET",
            "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects/download",
        ) => "bucket:read",
        ("GET", "/api/organization/{organization_id}/registry/repositories") => "registry:read",
        ("POST", "/api/organization/{organization_id}/registry/repositories") => "registry:create",
        ("DELETE", "/api/organization/{organization_id}/registry/repositories/{repository_id}") => {
            "registry:delete"
        }
        ("GET", "/api/organization/{organization_id}/registry/external-registries") => {
            "registry:read"
        }
        ("POST", "/api/organization/{organization_id}/registry/external-registries") => {
            "registry:create"
        }
        (
            "PATCH",
            "/api/organization/{organization_id}/registry/external-registries/{registry_id}",
        ) => "registry:update",
        (
            "POST",
            "/api/organization/{organization_id}/registry/external-registries/{registry_id}/rotate-token",
        ) => "registry:update",
        (
            "DELETE",
            "/api/organization/{organization_id}/registry/external-registries/{registry_id}",
        ) => "registry:delete",
        ("GET", "/api/organization/{organization_id}/registry/access-tokens") => {
            "access-token:read"
        }
        ("POST", "/api/organization/{organization_id}/registry/access-tokens") => {
            "access-token:create"
        }
        ("GET", "/api/organization/{organization_id}/registry/access-tokens/{token_id}") => {
            "access-token:read"
        }
        ("PATCH", "/api/organization/{organization_id}/registry/access-tokens/{token_id}") => {
            "access-token:update"
        }
        ("DELETE", "/api/organization/{organization_id}/registry/access-tokens/{token_id}") => {
            "access-token:delete"
        }
        _ => return None,
    })
}

/// Routes reachable by API keys without any scope. Everything else must either
/// appear in `required_scope` or API keys are denied (fail-closed).
const PUBLIC_API_ROUTES: &[(&str, &str)] = &[
    ("GET", "/health"),
    ("GET", "/api/registry/token"),
    (
        "GET",
        "/api/organization/{organization_id}/registry/maintenance",
    ),
];

fn api_key_scope_requirement(
    method: &str,
    path: &str,
) -> Result<Option<&'static str>, AppError> {
    if PUBLIC_API_ROUTES.contains(&(method, path)) {
        return Ok(None);
    }
    required_scope(method, path).map(Some).ok_or_else(|| {
        AppError::Forbidden(format!(
            "API keys cannot access {method} {path}: no scope is registered for this route"
        ))
    })
}

fn ip_allowed(allowed_ips: Option<&str>, peer_ip: Option<IpAddr>) -> bool {
    let Some(allowed) = allowed_ips.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    match peer_ip {
        Some(peer) => allowed
            .split(',')
            .filter_map(|entry| entry.trim().parse::<IpAddr>().ok())
            .any(|ip| ip == peer),
        None => false,
    }
}

pub struct AuthContext {
    pub tenant_db: TenantDatabase,
    pub auth: RequestAuthContext,
}

#[derive(Deserialize)]
struct BetterAuthSessionUser {
    id: String,
}

#[derive(Deserialize)]
struct BetterAuthSession {
    user: BetterAuthSessionUser,
}

#[derive(Debug)]
struct ApiKeyLookup {
    id: Uuid,
    organization_id: Uuid,
    scopes: HashSet<String>,
}

impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let matched_path = parts
            .extensions
            .get::<MatchedPath>()
            .map(|path| path.as_str().to_owned());
        let peer_ip = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|info| info.0.ip());
        let state = get_app_state();
        let identity_db = state.identity_db;
        let tenant_db_conn = state.tenant_db;

        let (organization_context, request_auth) = if let Some(raw_api_key) =
            extract_api_key_from_parts(parts).map(str::to_owned)
        {
            let api_key: ApiKeyLookup = resolve_api_key(&identity_db, &raw_api_key, peer_ip)
                .await?
                .ok_or_else(|| AppError::Unauthorized("Invalid API key".to_string()))?;
            let request_auth = RequestAuthContext {
                actor_id: api_key.id,
                api_key_scopes: Some(api_key.scopes),
            };
            if let Some(path) = matched_path.as_deref() {
                if let Some(scope) = api_key_scope_requirement(parts.method.as_str(), path)? {
                    request_auth.require_scope(scope)?;
                }
            }
            (
                OrganizationContext {
                    allowed_organizations: vec![api_key.organization_id],
                },
                request_auth,
            )
        } else {
            let cookie_header = parts
                .headers
                .get("cookie")
                .and_then(|h| h.to_str().ok())
                .ok_or_else(|| AppError::Unauthorized("Missing session cookie".to_string()))?;
            let actor_id = resolve_user_from_cookie(cookie_header).await?;
            let allowed_organizations = resolve_user_organizations(&identity_db, actor_id).await?;

            if allowed_organizations.is_empty() {
                return Err(AppError::Forbidden(
                    "User has no organization access".to_string(),
                ));
            }

            (
                OrganizationContext {
                    allowed_organizations,
                },
                RequestAuthContext {
                    actor_id,
                    api_key_scopes: None,
                },
            )
        };

        let tenant_db = TenantDatabase::new(tenant_db_conn, organization_context);

        Ok(AuthContext {
            tenant_db,
            auth: request_auth,
        })
    }
}

async fn resolve_user_from_cookie(cookie_header: &str) -> Result<Uuid, AppError> {
    let session_url = std::env::var("BETTER_AUTH_SESSION_URL")
        .unwrap_or_else(|_| "http://ui:3000/api/auth/get-session".to_string());

    let response = reqwest_client()
        .get(&session_url)
        .header("cookie", cookie_header)
        .header("Origin", "http://ui:3000")
        .send()
        .await
        .map_err(|err| {
            tracing::warn!(
                "Failed to reach Better Auth session endpoint at {}: {err}",
                session_url
            );
            AppError::Unauthorized(format!("Failed to resolve Better Auth session: {err}"))
        })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        tracing::warn!(
            "Better Auth session endpoint returned {} for session lookup: {}",
            status,
            body.chars().take(500).collect::<String>()
        );
        return Err(AppError::Unauthorized(
            "Session is invalid or expired".to_string(),
        ));
    }

    let body_text = response.text().await.map_err(|err| {
        tracing::warn!("Failed to read Better Auth session response body: {err}");
        AppError::Unauthorized("Invalid Better Auth session response".to_string())
    })?;

    let session: BetterAuthSession = serde_json::from_str(&body_text).map_err(|err| {
        tracing::warn!(
            "Failed to parse Better Auth session response: {err} (body: {})",
            &body_text[..body_text.len().min(300)]
        );
        AppError::Unauthorized("Invalid Better Auth session response".to_string())
    })?;

    Uuid::parse_str(&session.user.id).map_err(|_| {
        tracing::warn!("Invalid Better Auth user id format: {}", session.user.id);
        AppError::Unauthorized("Invalid Better Auth user id".to_string())
    })
}

fn extract_api_key_from_parts(parts: &Parts) -> Option<&str> {
    if let Some(value) = parts.headers.get("x-api-key").and_then(|h| h.to_str().ok())
        && !value.trim().is_empty()
    {
        return Some(value.trim());
    }

    let auth_header = parts
        .headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())?;

    auth_header
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|v| !v.is_empty())
}

fn hash_api_key(raw_api_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_api_key.as_bytes());
    hex::encode(hasher.finalize())
}

async fn resolve_user_organizations(
    app_db: &AppDatabase,
    actor_id: Uuid,
) -> Result<Vec<Uuid>, AppError> {
    let rows = app_db
        .0
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT organization_id FROM organization_member WHERE user_id = $1",
            vec![actor_id.into()],
        ))
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    let mut organizations = Vec::with_capacity(rows.len());
    for row in rows {
        let org_id = row
            .try_get::<Uuid>("", "organization_id")
            .map_err(|err| AppError::Internal(err.to_string()))?;
        organizations.push(org_id);
    }

    Ok(organizations)
}

async fn resolve_api_key(
    app_db: &AppDatabase,
    raw_api_key: &str,
    peer_ip: Option<IpAddr>,
) -> Result<Option<ApiKeyLookup>, AppError> {
    let key_hash = hash_api_key(raw_api_key);

    let key_row = app_db
        .0
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT api_keys.id, api_keys.organization_id, api_keys.allowed_ips, COALESCE(array_agg(api_key_scopes.scope::text) FILTER (WHERE api_key_scopes.scope IS NOT NULL), ARRAY[]::text[]) AS scopes FROM api_keys LEFT JOIN api_key_scopes ON api_key_scopes.api_key_id = api_keys.id WHERE api_keys.key_hash = $1 AND (api_keys.expires_at IS NULL OR api_keys.expires_at = 0 OR api_keys.created_at + make_interval(months => api_keys.expires_at) > NOW()) GROUP BY api_keys.id, api_keys.organization_id, api_keys.allowed_ips LIMIT 1",
            vec![key_hash.into()],
        ))
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    let Some(row) = key_row else {
        return Ok(None);
    };

    let key_id = row
        .try_get::<Uuid>("", "id")
        .map_err(|err| AppError::Internal(err.to_string()))?;
    let organization_id = row
        .try_get::<Uuid>("", "organization_id")
        .map_err(|err| AppError::Internal(err.to_string()))?;
    let allowed_ips: Option<String> = row
        .try_get::<Option<String>>("", "allowed_ips")
        .map_err(|err| AppError::Internal(err.to_string()))?;
    if !ip_allowed(allowed_ips.as_deref(), peer_ip) {
        tracing::warn!(
            "API key {key_id} rejected: peer IP {:?} not in allowlist",
            peer_ip
        );
        return Ok(None);
    }
    let scopes = row
        .try_get::<Vec<String>>("", "scopes")
        .map_err(|err| AppError::Internal(err.to_string()))?
        .into_iter()
        .collect();

    Ok(Some(ApiKeyLookup {
        id: key_id,
        organization_id,
        scopes,
    }))
}

#[cfg(test)]
mod tests {
    use super::RequestAuthContext;
    use std::collections::HashSet;
    use std::net::IpAddr;
    use uuid::Uuid;

    #[test]
    fn unlisted_routes_are_denied_for_api_keys() {
        assert!(super::api_key_scope_requirement("GET", "/health").unwrap().is_none());
        assert!(
            super::api_key_scope_requirement("GET", "/api/registry/token")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            super::api_key_scope_requirement(
                "GET",
                "/api/organization/{organization_id}/containers"
            )
            .unwrap(),
            Some("container:read")
        );
        // Not in the scope table and not public -> denied, not silently allowed.
        assert!(super::api_key_scope_requirement("DELETE", "/some/future/route").is_err());
    }

    #[test]
    fn ip_allowlist_is_enforced() {
        let ip = |s: &str| s.parse::<IpAddr>().unwrap();

        // Empty/missing allowlist allows everything.
        assert!(super::ip_allowed(None, Some(ip("1.2.3.4"))));
        assert!(super::ip_allowed(Some(""), Some(ip("1.2.3.4"))));
        assert!(super::ip_allowed(Some("  "), Some(ip("1.2.3.4"))));

        let list = "192.168.1.1, 10.0.0.1";
        assert!(super::ip_allowed(Some(list), Some(ip("10.0.0.1"))));
        assert!(!super::ip_allowed(Some(list), Some(ip("10.0.0.2"))));

        // No peer IP (e.g. unix socket / test harness) with an allowlist -> deny.
        assert!(!super::ip_allowed(Some(list), None));

        // Malformed entries are ignored, valid ones still match.
        assert!(super::ip_allowed(Some("not-an-ip, 10.0.0.1"), Some(ip("10.0.0.1"))));
    }

    #[test]
    fn api_key_scope_checks_fail_closed_while_sessions_bypass_them() {
        let session = RequestAuthContext {
            actor_id: Uuid::nil(),
            api_key_scopes: None,
        };
        assert!(session.require_scope("project:delete").is_ok());

        let scoped_key = RequestAuthContext {
            actor_id: Uuid::nil(),
            api_key_scopes: Some(HashSet::from(["project:read".into()])),
        };
        assert!(scoped_key.require_scope("project:read").is_ok());
        assert!(scoped_key.require_scope("project:delete").is_err());
    }

    #[test]
    fn every_protected_route_has_its_exact_scope() {
        let expected = [
            (
                "GET",
                "/api/organization/{organization_id}/regions",
                "region:read",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects",
                "project:read",
            ),
            (
                "POST",
                "/api/organization/{organization_id}/projects",
                "project:create",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}",
                "project:read",
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/projects/{project_id}",
                "project:delete",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/environments",
                "project:read",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}/environments",
                "project:read",
            ),
            (
                "POST",
                "/api/organization/{organization_id}/projects/{project_id}/environments",
                "project:manage",
            ),
            (
                "PATCH",
                "/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}",
                "project:manage",
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}",
                "project:manage",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}/timelines",
                "timeline:read",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}/timelines/{timeline_id}",
                "timeline:read",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/events",
                "event:read",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/containers",
                "container:read",
            ),
            (
                "POST",
                "/api/organization/{organization_id}/containers",
                "container:create",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/containers/{container_id}",
                "container:read",
            ),
            (
                "PATCH",
                "/api/organization/{organization_id}/containers/{container_id}",
                "container:update",
            ),
            (
                "POST",
                "/api/organization/{organization_id}/containers/{container_id}/deploy",
                "container:update",
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/containers/{container_id}",
                "container:delete",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/databases/postgres",
                "database:postgres:read",
            ),
            (
                "POST",
                "/api/organization/{organization_id}/databases/postgres",
                "database:postgres:create",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/databases/postgres/{database_id}",
                "database:postgres:read",
            ),
            (
                "PATCH",
                "/api/organization/{organization_id}/databases/postgres/{database_id}",
                "database:postgres:update",
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/databases/postgres/{database_id}",
                "database:postgres:delete",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/databases/postgres/{database_id}/branches",
                "database:postgres:read",
            ),
            (
                "POST",
                "/api/organization/{organization_id}/databases/postgres/{database_id}/branches",
                "database:postgres:manage",
            ),
            (
                "PATCH",
                "/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}",
                "database:postgres:manage",
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}",
                "database:postgres:manage",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens",
                "access-token:read",
            ),
            (
                "POST",
                "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens",
                "access-token:create",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
                "access-token:read",
            ),
            (
                "PATCH",
                "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
                "access-token:update",
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
                "access-token:delete",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/storage/buckets",
                "bucket:read",
            ),
            (
                "POST",
                "/api/organization/{organization_id}/storage/buckets",
                "bucket:create",
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/storage/buckets/{bucket_id}",
                "bucket:delete",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects",
                "bucket:read",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects/download",
                "bucket:read",
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects",
                "bucket:delete",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/registry/repositories",
                "registry:read",
            ),
            (
                "POST",
                "/api/organization/{organization_id}/registry/repositories",
                "registry:create",
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/registry/repositories/{repository_id}",
                "registry:delete",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/registry/external-registries",
                "registry:read",
            ),
            (
                "POST",
                "/api/organization/{organization_id}/registry/external-registries",
                "registry:create",
            ),
            (
                "PATCH",
                "/api/organization/{organization_id}/registry/external-registries/{registry_id}",
                "registry:update",
            ),
            (
                "POST",
                "/api/organization/{organization_id}/registry/external-registries/{registry_id}/rotate-token",
                "registry:update",
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/registry/external-registries/{registry_id}",
                "registry:delete",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/registry/access-tokens",
                "access-token:read",
            ),
            (
                "POST",
                "/api/organization/{organization_id}/registry/access-tokens",
                "access-token:create",
            ),
            (
                "GET",
                "/api/organization/{organization_id}/registry/access-tokens/{token_id}",
                "access-token:read",
            ),
            (
                "PATCH",
                "/api/organization/{organization_id}/registry/access-tokens/{token_id}",
                "access-token:update",
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/registry/access-tokens/{token_id}",
                "access-token:delete",
            ),
        ];

        for (method, path, scope) in expected {
            assert_eq!(
                super::required_scope(method, path),
                Some(scope),
                "{method} {path}"
            );
        }

        for (method, path) in [
            ("GET", "/health"),
            ("GET", "/api/registry/token"),
            (
                "GET",
                "/api/organization/{organization_id}/registry/maintenance",
            ),
        ] {
            assert_eq!(super::required_scope(method, path), None, "{method} {path}");
        }
    }
}
