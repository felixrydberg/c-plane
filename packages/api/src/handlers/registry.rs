use axum::{
    Json,
    extract::{Path, RawQuery},
    http::{HeaderMap, header},
};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::env;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{errors::AppError, state::get_app_state};

const REGISTRY_MAINTENANCE_MESSAGE: &str =
    "Registry is read-only for maintenance; retry after maintenance completes";

#[derive(Debug)]
pub struct RegistryTokenQuery {
    service: String,
    // ponytail: normal push/pull requests one repository; support repeated scopes if cross-repository blob mounts are needed.
    scope: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct RegistryTokenResponse {
    token: String,
    access_token: String,
    expires_in: u64,
    issued_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct RegistryMaintenanceResponse {
    pub read_only: bool,
    pub phase: Option<String>,
    pub started_at: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/registry/maintenance",
    params(("organization_id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Registry maintenance state", body = RegistryMaintenanceResponse),
    ),
    tag = "registry",
)]
pub async fn maintenance_status(
    Path(_organization_id): Path<Uuid>,
) -> Result<Json<RegistryMaintenanceResponse>, AppError> {
    let row = registry_maintenance_row().await?;
    let phase: String = row.try_get("", "phase").map_err(maintenance_error)?;
    let started_at = row
        .try_get::<Option<chrono::DateTime<chrono::Utc>>>("", "started_at")
        .map_err(maintenance_error)?
        .map(|value| value.to_rfc3339());
    Ok(Json(RegistryMaintenanceResponse {
        read_only: phase != "idle",
        phase: (phase != "idle").then_some(phase),
        started_at,
    }))
}

#[derive(Serialize)]
struct RegistryClaims {
    iss: String,
    sub: String,
    aud: String,
    exp: u64,
    nbf: u64,
    iat: u64,
    jti: String,
    access: Vec<RegistryAccess>,
}

#[derive(Serialize, Debug, PartialEq)]
struct RegistryAccess {
    #[serde(rename = "type")]
    resource_type: &'static str,
    name: String,
    actions: Vec<String>,
}

struct RegistryIdentity {
    id: Uuid,
    organization_id: Uuid,
}

#[utoipa::path(
    get,
    path = "/api/registry/token",
    params(
        ("service" = String, Query, description = "Distribution service name"),
        ("scope" = Vec<String>, Query, description = "Requested repository scopes"),
    ),
    responses(
        (status = 200, description = "Short-lived Distribution access token", body = RegistryTokenResponse),
        (status = 401, description = "Invalid registry credentials"),
        (status = 503, description = "Registry is read-only for maintenance", body = crate::errors::ErrorResponse),
    ),
    tag = "registry",
)]
pub async fn issue_token(
    headers: HeaderMap,
    RawQuery(raw_query): RawQuery,
) -> Result<Json<RegistryTokenResponse>, AppError> {
    let query = parse_registry_token_query(raw_query.as_deref())?;
    let expected_service = env::var("REGISTRY_HOST").unwrap_or_else(|_| "localhost:5000".into());
    if query.service != expected_service {
        return Err(AppError::Unauthorized("Unknown registry service".into()));
    }

    let (username, raw_token) = basic_credentials(&headers)?;
    let identity = resolve_registry_token(&raw_token)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid registry credentials".into()))?;
    let organization_slug = organization_slug(identity.organization_id).await?;
    if username != organization_slug {
        return Err(AppError::Unauthorized(
            "Invalid registry credentials".into(),
        ));
    }

    let read_only = registry_is_read_only().await?;
    if read_only && requests_write(&query.scope, &organization_slug) {
        return Err(AppError::ServiceUnavailable(
            REGISTRY_MAINTENANCE_MESSAGE.into(),
        ));
    }
    let token_ttl_seconds = get_app_state().config.registry_token_ttl_seconds;
    let mut access = Vec::new();
    for scope in query.scope {
        let Some(requested) = access_for_scope(&scope, &organization_slug) else {
            continue;
        };
        if let Some(authorized) =
            authorize_repository(requested, identity.id, identity.organization_id, read_only)
                .await?
        {
            access.push(authorized);
        }
    }
    let now = chrono::Utc::now();
    let issued_at = now.timestamp() as u64;
    let claims = RegistryClaims {
        iss: env::var("REGISTRY_TOKEN_ISSUER").unwrap_or_else(|_| "cplane-registry".into()),
        sub: identity.id.to_string(),
        aud: expected_service,
        exp: registry_token_exp(issued_at, token_ttl_seconds)?,
        nbf: issued_at.saturating_sub(5),
        iat: issued_at,
        jti: Uuid::new_v4().to_string(),
        access,
    };
    let token = sign_registry_claims(&claims)?;

    Ok(Json(RegistryTokenResponse {
        access_token: token.clone(),
        token,
        expires_in: token_ttl_seconds,
        issued_at: now.to_rfc3339(),
    }))
}

pub(crate) async fn sign_repository_token(
    organization_id: Uuid,
    repository_name: &str,
    actions: &[&str],
) -> Result<String, AppError> {
    if actions
        .iter()
        .any(|action| matches!(*action, "push" | "delete"))
        && registry_is_read_only().await?
    {
        return Err(AppError::ServiceUnavailable(
            REGISTRY_MAINTENANCE_MESSAGE.into(),
        ));
    }
    let organization_slug = organization_slug(organization_id).await?;
    let token_ttl_seconds = get_app_state().config.registry_token_ttl_seconds;
    let now = chrono::Utc::now();
    let issued_at = now.timestamp() as u64;
    sign_registry_claims(&RegistryClaims {
        iss: env::var("REGISTRY_TOKEN_ISSUER").unwrap_or_else(|_| "cplane-registry".into()),
        sub: "cplane-control-plane".into(),
        aud: env::var("REGISTRY_HOST").unwrap_or_else(|_| "localhost:5000".into()),
        exp: registry_token_exp(issued_at, token_ttl_seconds)?,
        nbf: issued_at.saturating_sub(5),
        iat: issued_at,
        jti: Uuid::new_v4().to_string(),
        access: vec![RegistryAccess {
            resource_type: "repository",
            name: format!("{organization_slug}/{repository_name}"),
            actions: actions.iter().map(|action| (*action).into()).collect(),
        }],
    })
}

fn sign_registry_claims(claims: &RegistryClaims) -> Result<String, AppError> {
    let secret = registry_signing_secret()?;
    sign_registry_claims_with_secret(claims, &secret)
}

fn sign_registry_claims_with_secret(
    claims: &RegistryClaims,
    secret: &[u8],
) -> Result<String, AppError> {
    let mut jwt_header = Header::new(Algorithm::HS256);
    jwt_header.kid = Some("cplane-registry".into());
    encode(&jwt_header, claims, &EncodingKey::from_secret(secret))
        .map_err(|error| AppError::Internal(format!("Failed to sign registry token: {error}")))
}

fn registry_token_exp(issued_at: u64, token_ttl_seconds: u64) -> Result<u64, AppError> {
    issued_at
        .checked_add(token_ttl_seconds)
        .ok_or_else(|| AppError::Internal("REGISTRY_TOKEN_TTL_SECONDS is too large".into()))
}

fn parse_registry_token_query(raw_query: Option<&str>) -> Result<RegistryTokenQuery, AppError> {
    let mut service = None;
    let mut scope = Vec::new();
    for (key, value) in form_urlencoded::parse(raw_query.unwrap_or_default().as_bytes()) {
        match key.as_ref() {
            "service" => service = Some(value.into_owned()),
            "scope" => scope.push(value.into_owned()),
            _ => {}
        }
    }
    Ok(RegistryTokenQuery {
        service: service
            .ok_or_else(|| AppError::Unauthorized("Registry service is required".into()))?,
        scope,
    })
}

fn basic_credentials(headers: &HeaderMap) -> Result<(String, String), AppError> {
    let encoded = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Basic "))
        .ok_or_else(|| AppError::Unauthorized("Registry credentials are required".into()))?;
    let decoded = STANDARD
        .decode(encoded)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or_else(|| AppError::Unauthorized("Invalid registry credentials".into()))?;
    let (username, password) = decoded
        .split_once(':')
        .filter(|(username, password)| !username.is_empty() && !password.is_empty())
        .ok_or_else(|| AppError::Unauthorized("Invalid registry credentials".into()))?;
    Ok((username.to_owned(), password.to_owned()))
}

fn access_for_scope(scope: &str, organization_slug: &str) -> Option<RegistryAccess> {
    let mut parts = scope.splitn(3, ':');
    if parts.next()? != "repository" {
        return None;
    }
    let name = parts.next()?;
    if !valid_repository_name(name, organization_slug) {
        return None;
    }
    let requested = parts.next()?.split(',');
    let actions = requested
        .filter(|action| matches!(*action, "pull" | "push" | "delete"))
        .map(str::to_owned)
        .collect();
    Some(RegistryAccess {
        resource_type: "repository",
        name: name.to_owned(),
        actions,
    })
}

fn requests_write(scopes: &[String], organization_slug: &str) -> bool {
    scopes.iter().any(|scope| {
        access_for_scope(scope, organization_slug).is_some_and(|access| {
            access
                .actions
                .iter()
                .any(|action| matches!(action.as_str(), "push" | "delete"))
        })
    })
}

fn valid_repository_name(name: &str, organization_slug: &str) -> bool {
    let mut segments = name.split('/');
    segments.next() == Some(organization_slug)
        && segments.clone().next().is_some()
        && segments.all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'.' | b'_' | b'-')
                })
        })
}

async fn authorize_repository(
    access: RegistryAccess,
    access_token_id: Uuid,
    organization_id: Uuid,
    read_only: bool,
) -> Result<Option<RegistryAccess>, AppError> {
    let repository_name = access
        .name
        .split_once('/')
        .map(|(_, name)| name)
        .ok_or_else(|| AppError::Unauthorized("Invalid repository scope".into()))?;
    let row = get_app_state()
        .identity_db
        .connection()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT grants.can_pull, grants.can_push FROM registry_repository_grants grants JOIN registry_repositories repositories ON repositories.id = grants.repository_id AND repositories.organization_id = grants.organization_id WHERE grants.access_token_id = $1 AND grants.organization_id = $2 AND repositories.name = $3 LIMIT 1",
            vec![access_token_id.into(), organization_id.into(), repository_name.into()],
        ))
        .await
        .map_err(|error| AppError::Internal(format!("Failed to authorize repository: {error}")))?;
    let Some(row) = row else {
        return Ok(None);
    };
    let can_pull = row
        .try_get::<bool>("", "can_pull")
        .map_err(|error| AppError::Internal(format!("Failed to authorize repository: {error}")))?;
    let can_push = row
        .try_get::<bool>("", "can_push")
        .map_err(|error| AppError::Internal(format!("Failed to authorize repository: {error}")))?;
    Ok(Some(apply_repository_grant(
        access, can_pull, can_push, read_only,
    )))
}

async fn registry_maintenance_row() -> Result<sea_orm::QueryResult, AppError> {
    get_app_state()
        .identity_db
        .connection()
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT COALESCE((SELECT phase FROM registry_maintenance WHERE service='distribution'), 'idle') AS phase, (SELECT started_at FROM registry_maintenance WHERE service='distribution') AS started_at",
        ))
        .await
        .map_err(maintenance_error)?
        .ok_or_else(|| AppError::Internal("Failed to read registry maintenance state".into()))
}

async fn registry_is_read_only() -> Result<bool, AppError> {
    registry_maintenance_row()
        .await?
        .try_get::<String>("", "phase")
        .map(|phase| phase != "idle")
        .map_err(maintenance_error)
}

fn maintenance_error(error: impl std::fmt::Display) -> AppError {
    AppError::Internal(format!(
        "Failed to read registry maintenance state: {error}"
    ))
}

async fn resolve_registry_token(raw_token: &str) -> Result<Option<RegistryIdentity>, AppError> {
    let token_hash = hex::encode(Sha256::digest(raw_token.as_bytes()));
    let row = get_app_state()
        .identity_db
        .connection()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, organization_id FROM registry_access_tokens WHERE token_hash = $1 AND revoked_at IS NULL LIMIT 1",
            vec![token_hash.into()],
        ))
        .await
        .map_err(|error| AppError::Internal(format!("Failed to resolve registry token: {error}")))?;
    row.map(|row| {
        Ok(RegistryIdentity {
            id: row.try_get("", "id").map_err(|error| {
                AppError::Internal(format!("Failed to resolve registry token: {error}"))
            })?,
            organization_id: row.try_get("", "organization_id").map_err(|error| {
                AppError::Internal(format!("Failed to resolve registry token: {error}"))
            })?,
        })
    })
    .transpose()
}

fn apply_repository_grant(
    mut access: RegistryAccess,
    can_pull: bool,
    can_push: bool,
    read_only: bool,
) -> RegistryAccess {
    access.actions.retain(|action| {
        (action == "pull" && can_pull)
            || (matches!(action.as_str(), "push" | "delete") && can_push && !read_only)
    });
    access
}

pub(crate) async fn organization_slug(organization_id: Uuid) -> Result<String, AppError> {
    let row = get_app_state()
        .identity_db
        .connection()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT slug FROM organization WHERE id = $1 LIMIT 1",
            vec![organization_id.into()],
        ))
        .await
        .map_err(|error| AppError::Internal(format!("Failed to resolve organization: {error}")))?
        .ok_or_else(|| AppError::Unauthorized("Invalid registry credentials".into()))?;
    row.try_get("", "slug")
        .map_err(|error| AppError::Internal(format!("Failed to resolve organization: {error}")))
}

fn registry_signing_secret() -> Result<Vec<u8>, AppError> {
    let encoded = env::var("REGISTRY_TOKEN_SECRET")
        .map_err(|_| AppError::Internal("REGISTRY_TOKEN_SECRET is required".into()))?;
    let secret = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| AppError::Internal("REGISTRY_TOKEN_SECRET is invalid".into()))?;
    if secret.len() < 32 {
        return Err(AppError::Internal(
            "REGISTRY_TOKEN_SECRET must contain at least 32 bytes".into(),
        ));
    }
    Ok(secret)
}

#[cfg(test)]
mod tests {
    use super::{
        RegistryAccess, RegistryClaims, access_for_scope, apply_repository_grant,
        parse_registry_token_query, registry_token_exp, requests_write,
        sign_registry_claims_with_secret,
    };
    use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
    use serde_json::Value;

    #[test]
    fn accepts_repeated_registry_scopes() {
        let query = parse_registry_token_query(Some(
            "service=localhost%3A5000&scope=repository%3Aacme%2Fapi%3Apull&scope=repository%3Aacme%2Fapi%3Apull%2Cpush",
        ))
        .unwrap();

        assert_eq!(query.scope.len(), 2);
    }

    #[test]
    fn identifies_write_token_requests() {
        assert!(requests_write(&["repository:acme/api:push".into()], "acme"));
        assert!(!requests_write(
            &["repository:acme/api:pull".into()],
            "acme"
        ));
    }

    #[test]
    fn rejects_token_expiration_overflow() {
        assert_eq!(registry_token_exp(1, 60).unwrap(), 61);
        assert!(registry_token_exp(u64::MAX, 1).is_err());
    }

    #[test]
    fn grants_only_the_tokens_organization_and_actions() {
        let access = access_for_scope("repository:acme/api:pull,push,delete", "acme").unwrap();
        assert_eq!(access.name, "acme/api");
        assert_eq!(access.actions, vec!["pull", "push", "delete"]);

        assert!(access_for_scope("repository:other/api:pull", "acme").is_none());

        let access = access_for_scope("repository:acme/api:pull,push,delete", "acme").unwrap();
        assert_eq!(
            apply_repository_grant(access, true, false, false).actions,
            vec!["pull"]
        );

        let access = access_for_scope("repository:acme/api:pull,push,delete", "acme").unwrap();
        assert_eq!(
            apply_repository_grant(access, false, true, false).actions,
            vec!["push", "delete"]
        );

        let access = access_for_scope("repository:acme/api:pull,push,delete", "acme").unwrap();
        assert_eq!(
            apply_repository_grant(access, true, true, true).actions,
            vec!["pull"]
        );
    }

    #[test]
    fn signs_tokens_verifiable_by_the_symmetric_registry_key() {
        let secret = [7_u8; 32];
        let token = sign_registry_claims_with_secret(
            &RegistryClaims {
                iss: "cplane-registry".into(),
                sub: "cplane-control-plane".into(),
                aud: "registry.example.com".into(),
                exp: 4_102_444_800,
                nbf: 0,
                iat: 0,
                jti: "test".into(),
                access: vec![RegistryAccess {
                    resource_type: "repository",
                    name: "acme/api".into(),
                    actions: vec!["pull".into()],
                }],
            },
            &secret,
        )
        .unwrap();
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&["registry.example.com"]);
        validation.set_issuer(&["cplane-registry"]);
        let claims = decode::<Value>(&token, &DecodingKey::from_secret(&secret), &validation)
            .unwrap()
            .claims;

        assert_eq!(claims["access"][0]["name"], "acme/api");
    }
}
