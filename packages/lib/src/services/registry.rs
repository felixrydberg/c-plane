use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, QueryFilter,
    Statement,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    entities::{project, secret},
    error::AppError,
    secrets::{self, Client},
};

#[derive(Debug)]
pub struct SignedRepositoryAccess {
    pub token: String,
    pub repository_name: String,
}

pub async fn organization_slug(
    database: &DatabaseConnection,
    organization_id: Uuid,
) -> Result<String, AppError> {
    let row = database
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT slug FROM organization WHERE id = $1 LIMIT 1",
            vec![organization_id.into()],
        ))
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid registry credentials".into()))?;
    row.try_get("", "slug")
        .map_err(|error| AppError::Internal(format!("Failed to resolve organization: {error}")))
}

pub async fn resolve_registry_project_id(
    database: &DatabaseConnection,
    organization_id: Uuid,
    project_name: &str,
) -> Result<Uuid, AppError> {
    let projects = project::Entity::find()
        .filter(project::Column::OrganizationId.eq(organization_id))
        .all(database)
        .await?;
    let matching_projects = projects
        .into_iter()
        .filter(|project| normalize_project_name(&project.name) == project_name)
        .collect::<Vec<_>>();
    match matching_projects.as_slice() {
        [project] => Ok(project.id),
        _ => Err(AppError::BadRequest(
            "Internal registry image has an invalid project name".into(),
        )),
    }
}

pub async fn sign_repository_access(
    database: &DatabaseConnection,
    organization_id: Uuid,
    project_id: Uuid,
    repository_id: Uuid,
    repository_name: &str,
    actions: &[&str],
    token_ttl_seconds: u64,
) -> Result<SignedRepositoryAccess, AppError> {
    let organization_slug = organization_slug(database, organization_id).await?;
    let project_name = registry_project_name(database, organization_id, project_id).await?;
    let repository_name = format!("{organization_slug}/{project_name}/{repository_name}");
    let now = Utc::now();
    let issued_at = now.timestamp() as u64;
    let token = sign_registry_claims(&RegistryClaims {
        iss: std::env::var("REGISTRY_TOKEN_ISSUER").unwrap_or_else(|_| "cplane-registry".into()),
        sub: "cplane-control-plane".into(),
        aud: std::env::var("REGISTRY_HOST").unwrap_or_else(|_| "localhost:5000".into()),
        exp: issued_at
            .checked_add(token_ttl_seconds)
            .ok_or_else(|| AppError::Internal("REGISTRY_TOKEN_TTL_SECONDS is too large".into()))?,
        nbf: issued_at.saturating_sub(5),
        iat: issued_at,
        jti: Uuid::new_v4().to_string(),
        organization_id,
        access: vec![RegistryAccess {
            resource_type: "repository",
            name: repository_name.clone(),
            actions: actions.iter().map(|action| (*action).into()).collect(),
            repository_id: Some(repository_id),
        }],
    })?;
    Ok(SignedRepositoryAccess {
        token,
        repository_name,
    })
}

pub async fn load_secret(
    database: &DatabaseConnection,
    secrets_client: &Client,
    organization_id: Uuid,
    registry_id: Uuid,
) -> Result<String, AppError> {
    let row = secret::Entity::find_by_id(registry_id)
        .filter(secret::Column::OrganizationId.eq(organization_id))
        .one(database)
        .await?
        .filter(|row| row.scope == secret::SecretScope::Tenant)
        .filter(|row| row.organization_id == Some(organization_id))
        .ok_or_else(|| {
            AppError::Conflict("External registry credentials are unavailable".into())
        })?;
    let plaintext = secrets::decrypt(
        secrets_client,
        &format!("tenant-{}", organization_id.simple()),
        &row.ciphertext,
    )
    .await?;
    let secret: ExternalRegistrySecret = serde_json::from_slice(&plaintext)
        .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(secret.token)
}

pub fn normalize_project_name(name: &str) -> String {
    let mut normalized = String::new();
    let mut pending_separator = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if pending_separator && !normalized.is_empty() {
                normalized.push('-');
            }
            normalized.push(character.to_ascii_lowercase());
            pending_separator = false;
        } else if !normalized.is_empty() {
            pending_separator = true;
        }
    }
    if normalized.is_empty() {
        "project".into()
    } else {
        normalized
    }
}

async fn registry_project_name(
    database: &DatabaseConnection,
    organization_id: Uuid,
    project_id: Uuid,
) -> Result<String, AppError> {
    project::Entity::find_by_id(project_id)
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(database)
        .await?
        .map(|project| normalize_project_name(&project.name))
        .ok_or_else(|| AppError::NotFound("Project not found".into()))
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
    organization_id: Uuid,
    access: Vec<RegistryAccess>,
}

#[derive(Serialize)]
struct RegistryAccess {
    #[serde(rename = "type")]
    resource_type: &'static str,
    name: String,
    actions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository_id: Option<Uuid>,
}

#[derive(serde::Deserialize)]
struct ExternalRegistrySecret {
    token: String,
}

fn sign_registry_claims(claims: &RegistryClaims) -> Result<String, AppError> {
    let encoded = std::env::var("REGISTRY_TOKEN_SECRET")
        .map_err(|_| AppError::Internal("REGISTRY_TOKEN_SECRET is required".into()))?;
    let secret = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| AppError::Internal("REGISTRY_TOKEN_SECRET is invalid".into()))?;
    if secret.len() < 32 {
        return Err(AppError::Internal(
            "REGISTRY_TOKEN_SECRET must contain at least 32 bytes".into(),
        ));
    }
    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("cplane-registry".into());
    encode(&header, claims, &EncodingKey::from_secret(&secret))
        .map_err(|error| AppError::Internal(format!("Failed to sign registry token: {error}")))
}
