use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use reqwest::Client;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::errors::{AppError, DatabaseError};
use crate::state::{AppDatabase, OrganizationContext, TenantDatabase, get_app_state};

#[derive(Clone, Debug)]
pub struct RequestAuthContext {
    pub actor_id: Uuid,
    pub api_key_id: Option<Uuid>,
    pub scopes: Vec<String>,
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
    scopes: Vec<String>,
}

pub async fn tenant_auth_middleware(
    request: Request,
    next: Next,
) -> Response {
    let state = get_app_state();
    match tenant_auth_middleware_inner(
        state.identity_db,
        state.tenant_db,
        request,
        next,
    )
    .await
    {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn tenant_auth_middleware_inner(
    identity_db: AppDatabase,
    tenant_db: DatabaseConnection,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let (organization_context, request_auth) = if let Some(raw_api_key) = extract_api_key(&request).map(str::to_owned) {
        let api_key: ApiKeyLookup = resolve_api_key(&identity_db, &raw_api_key).await?.ok_or_else(|| {
            AppError::Unauthorized("Invalid API key".to_string())
        })?;

        (
            OrganizationContext {
                allowed_organizations: vec![api_key.organization_id],
                actor_id: api_key.id,
            },
            RequestAuthContext {
                actor_id: api_key.id,
                api_key_id: Some(api_key.id),
                scopes: api_key.scopes,
            },
        )
    } else {
        let actor_id = resolve_user_from_request(&request).await?;
        let allowed_organizations = resolve_user_organizations(&identity_db, actor_id).await?;

        if allowed_organizations.is_empty() {
            return Err(AppError::Forbidden(
                "User has no organization access".to_string(),
            ));
        }

        (
            OrganizationContext {
                allowed_organizations,
                actor_id,
            },
            RequestAuthContext {
                actor_id,
                api_key_id: None,
                scopes: vec![],
            },
        )
    };

    let tenant_db = TenantDatabase::new(tenant_db, organization_context);
    request.extensions_mut().insert(request_auth);
    request.extensions_mut().insert(tenant_db);

    Ok(next.run(request).await)
}

async fn resolve_user_from_request(request: &Request) -> Result<Uuid, AppError> {
    let cookie_header = request
        .headers()
        .get("cookie")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing session cookie".to_string()))?
        .to_owned();

    // Better Auth verification lives behind this endpoint; response must include user.id.
    let session_url = std::env::var("BETTER_AUTH_SESSION_URL")
        .unwrap_or_else(|_| "http://ui:3000/api/auth/get-session".to_string());

    let response = Client::new()
        .get(session_url)
        .header("cookie", cookie_header)
        .send()
        .await
        .map_err(|err| AppError::Unauthorized(format!("Failed to resolve Better Auth session: {err}")))?;

    if !response.status().is_success() {
        return Err(AppError::Unauthorized(
            "Session is invalid or expired".to_string(),
        ));
    }

    let session = response
        .json::<BetterAuthSession>()
        .await
        .map_err(|_| AppError::Unauthorized("Invalid Better Auth session response".to_string()))?;

    Uuid::parse_str(&session.user.id)
        .map_err(|_| AppError::Unauthorized("Invalid Better Auth user id".to_string()))
}

fn extract_api_key(request: &Request) -> Option<&str> {
    if let Some(value) = request
        .headers()
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
    {
        if !value.trim().is_empty() {
            return Some(value.trim());
        }
    }

    let auth_header = request
        .headers()
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

async fn resolve_user_organizations(app_db: &AppDatabase, actor_id: Uuid) -> Result<Vec<Uuid>, AppError> {
    let rows = app_db
        .0
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT organization_id FROM organization_member WHERE user_id = $1",
            vec![actor_id.into()],
        ))
        .await
        .map_err(|err| AppError::Database(DatabaseError::QueryFailed(err.to_string())))?;

    let mut organizations = Vec::with_capacity(rows.len());
    for row in rows {
        let org_id = row
            .try_get::<Uuid>("", "organization_id")
            .map_err(|err| AppError::Database(DatabaseError::QueryFailed(err.to_string())))?;
        organizations.push(org_id);
    }

    Ok(organizations)
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
            "SELECT id, organization_id FROM api_keys WHERE key_hash = $1 AND (expires_at IS NULL OR expires_at > EXTRACT(EPOCH FROM NOW())::int) LIMIT 1",
            vec![key_hash.into()],
        ))
        .await
        .map_err(|err| AppError::Database(DatabaseError::QueryFailed(err.to_string())))?;

    let Some(row) = key_row else {
        return Ok(None);
    };

    let key_id = row
        .try_get::<Uuid>("", "id")
        .map_err(|err| AppError::Database(DatabaseError::QueryFailed(err.to_string())))?;
    let organization_id = row
        .try_get::<Uuid>("", "organization_id")
        .map_err(|err| AppError::Database(DatabaseError::QueryFailed(err.to_string())))?;

    let scope_rows = app_db
        .0
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT scope FROM api_key_scopes WHERE api_key_id = $1",
            vec![key_id.into()],
        ))
        .await
        .map_err(|err| AppError::Database(DatabaseError::QueryFailed(err.to_string())))?;

    let mut scopes = Vec::with_capacity(scope_rows.len());
    for row in scope_rows {
        let scope = row
            .try_get::<String>("", "scope")
            .map_err(|err| AppError::Database(DatabaseError::QueryFailed(err.to_string())))?;
        scopes.push(scope);
    }

    Ok(Some(ApiKeyLookup {
        id: key_id,
        organization_id,
        scopes,
    }))
}
