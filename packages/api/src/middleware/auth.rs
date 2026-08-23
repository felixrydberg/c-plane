use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;
use reqwest::Client;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, SocketAddr};
use std::sync::OnceLock;
use std::time::Duration;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::scoped::{self, Role, RouteGuard};
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
    roles: HashMap<Uuid, Role>,
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
        let guard = parts.extensions.get::<RouteGuard>().copied();
        let request_path = parts.uri.path().to_owned();
        let peer_ip = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|info| info.0.ip());
        let state = get_app_state();
        let identity_db = state.identity_db;
        let tenant_db_conn = state.tenant_db;

        let (organization_context, request_auth) =
            if let Some(raw_api_key) = extract_api_key_from_parts(parts).map(str::to_owned) {
                // Routes without a declared scope deny API keys (fail-closed).
                let api_key: ApiKeyLookup = resolve_api_key(&identity_db, &raw_api_key, peer_ip)
                    .await?
                    .ok_or_else(|| AppError::Unauthorized("Invalid API key".to_string()))?;
                scoped::check_api_key(guard, &api_key.scopes)?;
                (
                    OrganizationContext {
                        allowed_organizations: vec![api_key.organization_id],
                    },
                    RequestAuthContext {
                        actor_id: api_key.id,
                        roles: HashMap::new(),
                    },
                )
            } else {
                let cookie_header = parts
                    .headers
                    .get("cookie")
                    .and_then(|h| h.to_str().ok())
                    .ok_or_else(|| AppError::Unauthorized("Missing session cookie".to_string()))?;
                let actor_id = resolve_user_from_cookie(cookie_header).await?;
                let memberships = resolve_user_memberships(&identity_db, actor_id).await?;

                if memberships.is_empty() {
                    return Err(AppError::Forbidden(
                        "User has no organization access".to_string(),
                    ));
                }

                let mut roles = HashMap::with_capacity(memberships.len());
                for (organization_id, role) in &memberships {
                    roles.insert(*organization_id, Role::parse(role));
                }
                let request_auth = RequestAuthContext { actor_id, roles };

                if let Some(guard) = guard {
                    scoped::check_role(guard, &request_path, &request_auth.roles)?;
                }

                (
                    OrganizationContext {
                        allowed_organizations: memberships
                            .into_iter()
                            .map(|(organization_id, _)| organization_id)
                            .collect(),
                    },
                    request_auth,
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

async fn resolve_user_memberships(
    app_db: &AppDatabase,
    actor_id: Uuid,
) -> Result<Vec<(Uuid, String)>, AppError> {
    let rows = app_db
        .0
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT organization_id, role FROM organization_member WHERE user_id = $1",
            vec![actor_id.into()],
        ))
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    let mut memberships = Vec::with_capacity(rows.len());
    for row in rows {
        let organization_id = row
            .try_get::<Uuid>("", "organization_id")
            .map_err(|err| AppError::Internal(err.to_string()))?;
        let role = row
            .try_get::<String>("", "role")
            .map_err(|err| AppError::Internal(err.to_string()))?;
        memberships.push((organization_id, role));
    }

    Ok(memberships)
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
    use super::ip_allowed;
    use std::net::IpAddr;

    #[test]
    fn ip_allowlist_is_enforced() {
        let ip = |s: &str| s.parse::<IpAddr>().unwrap();

        // Empty/missing allowlist allows everything.
        assert!(ip_allowed(None, Some(ip("1.2.3.4"))));
        assert!(ip_allowed(Some(""), Some(ip("1.2.3.4"))));
        assert!(ip_allowed(Some("  "), Some(ip("1.2.3.4"))));

        let list = "192.168.1.1, 10.0.0.1";
        assert!(ip_allowed(Some(list), Some(ip("10.0.0.1"))));
        assert!(!ip_allowed(Some(list), Some(ip("10.0.0.2"))));

        // No peer IP (e.g. unix socket / test harness) with an allowlist -> deny.
        assert!(!ip_allowed(Some(list), None));

        // Malformed entries are ignored, valid ones still match.
        assert!(ip_allowed(
            Some("not-an-ip, 10.0.0.1"),
            Some(ip("10.0.0.1"))
        ));
    }
}
