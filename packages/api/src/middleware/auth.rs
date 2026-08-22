use axum::extract::{FromRequestParts, MatchedPath};
use axum::http::request::Parts;
use reqwest::Client;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use std::sync::OnceLock;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationRole {
    Owner,
    Member,
}

#[derive(Clone, Debug)]
pub struct OrganizationAccess {
    pub role: OrganizationRole,
    pub scopes: HashSet<String>,
}

impl OrganizationAccess {
    pub fn allows(&self, scope: &str) -> bool {
        self.role == OrganizationRole::Owner || self.scopes.contains(scope)
    }
}

#[derive(Clone, Debug)]
pub struct RequestAuthContext {
    pub actor_id: Uuid,
    organizations: HashMap<Uuid, OrganizationAccess>,
}

impl RequestAuthContext {
    pub fn is_owner(&self, organization_id: Uuid) -> bool {
        self.organizations
            .get(&organization_id)
            .is_some_and(|access| access.role == OrganizationRole::Owner)
    }

    pub fn has_scope(&self, organization_id: Uuid, scope: &str) -> bool {
        self.organizations
            .get(&organization_id)
            .is_some_and(|access| access.allows(scope))
    }

    pub fn require_scope(&self, organization_id: Uuid, scope: &str) -> Result<(), AppError> {
        if self.has_scope(organization_id, scope) {
            return Ok(());
        }
        Err(AppError::Forbidden(format!(
            "Missing required permission: {scope}"
        )))
    }

    #[allow(dead_code)]
    pub fn require_owner(&self, organization_id: Uuid) -> Result<(), AppError> {
        if self.is_owner(organization_id) {
            return Ok(());
        }
        Err(AppError::Forbidden(
            "Only organization owners can perform this action".to_string(),
        ))
    }
}

fn organization_id_from_path(path: &str) -> Option<Uuid> {
    let rest = path.strip_prefix("/api/organization/")?;
    let segment = rest.split('/').next()?;
    Uuid::parse_str(segment).ok()
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
        ("GET", "/api/organization/{organization_id}/registry/maintenance") => "registry:read",
        _ => return None,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RoutePolicy {
    Public,
    Scope(&'static str),
    OwnerOnly,
}

pub(crate) fn route_policy(method: &str, path: &str) -> RoutePolicy {
    let method = if method == "HEAD" { "GET" } else { method };
    match required_scope(method, path) {
        Some(scope) => RoutePolicy::Scope(scope),
        None if path.starts_with("/api/organization/") => RoutePolicy::OwnerOnly,
        None => RoutePolicy::Public,
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

#[derive(Debug)]
struct MemberLookup {
    organization_id: Uuid,
    role: String,
    scopes: HashSet<String>,
}

impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let policy = parts
            .extensions
            .get::<MatchedPath>()
            .map(|path| route_policy(parts.method.as_str(), path.as_str()))
            .unwrap_or(RoutePolicy::Public);
        let state = get_app_state();
        let identity_db = state.identity_db;
        let tenant_db_conn = state.tenant_db;

        let (organization_context, request_auth) = if let Some(raw_api_key) =
            extract_api_key_from_parts(parts).map(str::to_owned)
        {
            let api_key: ApiKeyLookup = resolve_api_key(&identity_db, &raw_api_key)
                .await?
                .ok_or_else(|| AppError::Unauthorized("Invalid API key".to_string()))?;
            let mut organizations = HashMap::new();
            organizations.insert(
                api_key.organization_id,
                OrganizationAccess {
                    role: OrganizationRole::Member,
                    scopes: api_key.scopes,
                },
            );
            (
                OrganizationContext {
                    allowed_organizations: vec![api_key.organization_id],
                },
                RequestAuthContext {
                    actor_id: api_key.id,
                    organizations,
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

            let mut organizations = HashMap::new();
            let mut allowed_organizations = Vec::with_capacity(memberships.len());
            for membership in memberships {
                allowed_organizations.push(membership.organization_id);
                organizations.insert(
                    membership.organization_id,
                    OrganizationAccess {
                        role: if membership.role == "owner" {
                            OrganizationRole::Owner
                        } else {
                            OrganizationRole::Member
                        },
                        scopes: membership.scopes,
                    },
                );
            }

            (
                OrganizationContext {
                    allowed_organizations,
                },
                RequestAuthContext {
                    actor_id,
                    organizations,
                },
            )
        };

        if policy != RoutePolicy::Public {
            let organization_id = organization_id_from_path(parts.uri.path()).ok_or_else(|| {
                AppError::Forbidden("Route requires an organization context".to_string())
            })?;
            match policy {
                RoutePolicy::Scope(scope) => request_auth.require_scope(organization_id, scope)?,
                _ => request_auth.require_owner(organization_id)?,
            }
        }

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
) -> Result<Vec<MemberLookup>, AppError> {
    let rows = app_db
        .0
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT m.organization_id, m.role, COALESCE(array_agg(p.scope::text) FILTER (WHERE p.scope IS NOT NULL), ARRAY[]::text[]) AS scopes FROM organization_member m LEFT JOIN organization_member_permission p ON p.member_id = m.id WHERE m.user_id = $1 GROUP BY m.id, m.organization_id, m.role",
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
        let scopes = row
            .try_get::<Vec<String>>("", "scopes")
            .map_err(|err| AppError::Internal(err.to_string()))?
            .into_iter()
            .collect();
        memberships.push(MemberLookup {
            organization_id,
            role,
            scopes,
        });
    }

    Ok(memberships)
}

async fn resolve_api_key(
    app_db: &AppDatabase,
    raw_api_key: &str,
) -> Result<Option<ApiKeyLookup>, AppError> {
    let key_hash = hash_api_key(raw_api_key);

    let key_row = app_db
        .0
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT api_keys.id, api_keys.organization_id, COALESCE(array_agg(api_key_scopes.scope::text) FILTER (WHERE api_key_scopes.scope IS NOT NULL), ARRAY[]::text[]) AS scopes FROM api_keys LEFT JOIN api_key_scopes ON api_key_scopes.api_key_id = api_keys.id WHERE api_keys.key_hash = $1 AND (api_keys.expires_at IS NULL OR api_keys.expires_at = 0 OR api_keys.created_at + make_interval(months => api_keys.expires_at) > NOW()) GROUP BY api_keys.id, api_keys.organization_id LIMIT 1",
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
    use super::{OrganizationAccess, OrganizationRole, RequestAuthContext};
    use std::collections::HashMap;
    use uuid::Uuid;

    fn context(
        organizations: Vec<(Uuid, OrganizationRole, &[&str])>,
    ) -> RequestAuthContext {
        let mut map = HashMap::new();
        for (id, role, scopes) in organizations {
            map.insert(
                id,
                OrganizationAccess {
                    role,
                    scopes: scopes.iter().map(|s| s.to_string()).collect(),
                },
            );
        }
        RequestAuthContext {
            actor_id: Uuid::nil(),
            organizations: map,
        }
    }

    #[test]
    fn api_keys_are_scope_limited_members_of_one_organization() {
        let org = Uuid::new_v4();
        let other = Uuid::new_v4();
        let key = context(vec![(org, OrganizationRole::Member, &["project:read"])]);

        assert!(key.require_scope(org, "project:read").is_ok());
        assert!(key.require_scope(org, "project:delete").is_err());
        assert!(!key.is_owner(org), "api keys are never owners");

        assert!(key.require_scope(other, "project:read").is_err());
    }

    #[test]
    fn members_are_enforced_against_their_granted_scopes() {
        let org = Uuid::new_v4();
        let member = context(vec![
            (org, OrganizationRole::Member, &["container:read", "bucket:read"]),
        ]);

        assert!(member.has_scope(org, "container:read"));
        assert!(member.require_scope(org, "container:create").is_err());

        let promoted = context(vec![
            (
                org,
                OrganizationRole::Member,
                &["container:read", "container:create"],
            ),
        ]);
        assert!(promoted.require_scope(org, "container:create").is_ok());
    }

    #[test]
    fn owners_hold_every_scope_and_pass_owner_checks() {
        let org = Uuid::new_v4();
        let owner = context(vec![(org, OrganizationRole::Owner, &[])]);

        for scope in [
            "project:delete",
            "registry:update",
            "database:postgres:manage",
            "org:update",
        ] {
            assert!(owner.require_scope(org, scope).is_ok(), "{scope}");
        }
        assert!(owner.require_owner(org).is_ok());
    }

    #[test]
    fn members_fail_owner_checks_even_with_scopes() {
        let org = Uuid::new_v4();
        let member = context(vec![(
            org,
            OrganizationRole::Member,
            &[
                "org:update",
                "member:invite",
                "member:remove",
                "api-key:manage",
            ],
        )]);

        assert!(member.require_owner(org).is_err());
    }

    #[test]
    fn access_is_isolated_per_organization() {
        let org_a = Uuid::new_v4();
        let org_b = Uuid::new_v4();
        let user = context(vec![
            (org_a, OrganizationRole::Owner, &[]),
            (org_b, OrganizationRole::Member, &["project:read"]),
        ]);

        assert!(user.is_owner(org_a));
        assert!(user.require_scope(org_b, "project:read").is_ok());
        assert!(user.require_scope(org_b, "project:delete").is_err());
        assert!(user.require_owner(org_b).is_err());
    }

    #[test]
    fn unknown_organizations_fail_closed() {
        let org = Uuid::new_v4();
        let stranger = context(vec![]);
        assert!(stranger.require_scope(org, "project:read").is_err());
        assert!(stranger.require_owner(org).is_err());
        assert!(!stranger.is_owner(org));
    }

    #[test]
    fn organization_id_parses_from_request_paths() {
        let id = Uuid::new_v4();
        assert_eq!(
            super::organization_id_from_path(&format!("/api/organization/{id}/projects")),
            Some(id)
        );
        assert_eq!(
            super::organization_id_from_path(&format!("/api/organization/{id}")),
            Some(id)
        );
        assert_eq!(super::organization_id_from_path("/health"), None);
        assert_eq!(
            super::organization_id_from_path("/api/organization/not-a-uuid/projects"),
            None
        );
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

        for (method, path) in [("GET", "/health"), ("GET", "/api/registry/token")] {
            assert_eq!(super::required_scope(method, path), None, "{method} {path}");
        }
    }

    #[test]
    fn route_policy_fails_closed_and_normalizes_head() {
        use super::{RoutePolicy, route_policy};

        assert_eq!(
            route_policy("GET", "/api/organization/{organization_id}/projects"),
            RoutePolicy::Scope("project:read")
        );
        assert_eq!(
            route_policy(
                "DELETE",
                "/api/organization/{organization_id}/containers/{container_id}"
            ),
            RoutePolicy::Scope("container:delete")
        );

        assert_eq!(
            route_policy("HEAD", "/api/organization/{organization_id}/projects"),
            RoutePolicy::Scope("project:read")
        );

        assert_eq!(
            route_policy("GET", "/api/organization/{organization_id}/registry/maintenance"),
            RoutePolicy::Scope("registry:read")
        );

        assert_eq!(
            route_policy(
                "GET",
                "/api/organization/{organization_id}/something-new"
            ),
            RoutePolicy::OwnerOnly
        );
        assert_eq!(
            route_policy(
                "POST",
                "/api/organization/{organization_id}/registry/maintenance/run"
            ),
            RoutePolicy::OwnerOnly
        );

        assert_eq!(route_policy("GET", "/health"), RoutePolicy::Public);
        assert_eq!(
            route_policy("GET", "/api/registry/token"),
            RoutePolicy::Public
        );
    }
}
