use std::{env, time::Duration};

use oci_client::{
    Client, Reference,
    client::{ClientConfig, ClientProtocol},
    errors::OciDistributionError,
    secrets::RegistryAuth,
};
use reqwest::Url;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::{
    errors::AppError,
    handlers::{
        external_registries::load_secret,
        registry::{organization_slug, require_managed_registry, resolve_registry_project_id, sign_repository_access},
    },
    models::entities::{external_registry, registry_repository},
    state::get_app_state,
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn resolve_image(
    configured: &str,
    organization_id: Uuid,
    registry: Option<&external_registry::Model>,
) -> Result<String, AppError> {
    let configured = configured.trim();
    let reference: Reference = configured
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid image reference".into()))?;

    let registry_host = env::var("REGISTRY_HOST").unwrap_or_else(|_| "localhost:5000".into());
    let (network_reference, auth, client) =
        if reference.registry().eq_ignore_ascii_case(&registry_host) {
            if registry.is_some() {
                return Err(AppError::BadRequest(
                    "Internal images cannot use an external registry credential".into(),
                ));
            }
            internal_registry(&reference, organization_id).await?
        } else {
            external_registry(&reference, organization_id, registry).await?
        };

    if let Some(digest) = reference.digest() {
        return Ok(reference.clone_with_digest(digest.into()).whole());
    }

    let digest = client
        .fetch_manifest_digest(&network_reference, &auth)
        .await
        .map_err(map_oci_error)?;

    Ok(reference.clone_with_digest(digest).whole())
}

async fn internal_registry(
    reference: &Reference,
    organization_id: Uuid,
) -> Result<(Reference, RegistryAuth, Client), AppError> {
    require_managed_registry(organization_id).await?;
    let slug = organization_slug(organization_id).await?;
    let (project_name, repository_name) = internal_repository_name(reference.repository(), &slug)?;
    let project_id = resolve_registry_project_id(organization_id, project_name).await?;
    let repository = registry_repository::Entity::find()
        .filter(registry_repository::Column::OrganizationId.eq(organization_id))
        .filter(registry_repository::Column::ProjectId.eq(project_id))
        .filter(registry_repository::Column::Name.eq(repository_name))
        .one(get_app_state().identity_db.connection())
        .await?
        .ok_or_else(|| AppError::BadRequest("Internal registry repository was not found".into()))?;
    let access = sign_repository_access(
        organization_id,
        project_id,
        repository.id,
        repository_name,
        &["pull"],
    )
    .await?;
    let internal_url =
        env::var("REGISTRY_INTERNAL_URL").unwrap_or_else(|_| "http://registry:5000".into());
    let (registry, protocol) = internal_endpoint(&internal_url)?;
    let network_reference = match (reference.tag(), reference.digest()) {
        (Some(tag), None) => {
            Reference::with_tag(registry.clone(), access.repository_name.clone(), tag.into())
        }
        (None, Some(digest)) => Reference::with_digest(
            registry.clone(),
            access.repository_name.clone(),
            digest.into(),
        ),
        _ => unreachable!("parsed image references must have a tag or digest"),
    };
    let client = oci_client(protocol)?;
    Ok((
        network_reference,
        RegistryAuth::Bearer(access.token),
        client,
    ))
}

async fn external_registry(
    reference: &Reference,
    organization_id: Uuid,
    registry: Option<&external_registry::Model>,
) -> Result<(Reference, RegistryAuth, Client), AppError> {
    validate_external_registry_host(reference, registry)?;
    let auth = match registry {
        Some(registry) => RegistryAuth::Basic(
            registry.username.clone(),
            load_secret(organization_id, registry.id).await?,
        ),
        None => RegistryAuth::Anonymous,
    };
    let client = oci_client(ClientProtocol::Https)?;
    Ok((reference.clone(), auth, client))
}

fn internal_repository_name<'a>(
    repository: &'a str,
    slug: &str,
) -> Result<(&'a str, &'a str), AppError> {
    let scoped = repository
        .strip_prefix(&format!("{slug}/"))
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            AppError::BadRequest("Internal registry image must belong to this organization".into())
        })?;
    let (project_name, repository_name) = scoped.split_once('/').ok_or_else(|| {
        AppError::BadRequest("Internal registry image must include a project name".into())
    })?;
    if repository_name.is_empty() {
        return Err(AppError::BadRequest(
            "Internal registry repository is required".into(),
        ));
    }
    Ok((project_name, repository_name))
}

fn validate_external_registry_host(
    reference: &Reference,
    registry: Option<&external_registry::Model>,
) -> Result<(), AppError> {
    if let Some(registry) = registry {
        if !reference.registry().eq_ignore_ascii_case(&registry.host) {
            return Err(AppError::BadRequest(format!(
                "Selected registry host '{}' does not match image host '{}'",
                registry.host,
                reference.registry()
            )));
        }
    }
    Ok(())
}

fn oci_client(protocol: ClientProtocol) -> Result<Client, AppError> {
    Client::try_from(ClientConfig {
        protocol,
        read_timeout: Some(REQUEST_TIMEOUT),
        connect_timeout: Some(CONNECT_TIMEOUT),
        ..Default::default()
    })
    .map_err(|error| AppError::Internal(format!("Image resolver setup failed: {error}")))
}

fn internal_endpoint(value: &str) -> Result<(String, ClientProtocol), AppError> {
    let url = Url::parse(value)
        .map_err(|_| AppError::Internal("REGISTRY_INTERNAL_URL is invalid".into()))?;
    let protocol = match url.scheme() {
        "http" => ClientProtocol::Http,
        "https" => ClientProtocol::Https,
        _ => {
            return Err(AppError::Internal(
                "REGISTRY_INTERNAL_URL must use HTTP or HTTPS".into(),
            ));
        }
    };
    if url.username() != ""
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
    {
        return Err(AppError::Internal(
            "REGISTRY_INTERNAL_URL must be an origin without credentials, path, query, or fragment"
                .into(),
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| AppError::Internal("REGISTRY_INTERNAL_URL requires a host".into()))?;
    if host.contains(':') {
        return Err(AppError::Internal(
            "REGISTRY_INTERNAL_URL does not support an IPv6 literal".into(),
        ));
    }
    let registry = url
        .port()
        .map_or_else(|| host.to_string(), |port| format!("{host}:{port}"));
    Ok((registry, protocol))
}

fn map_oci_error(error: OciDistributionError) -> AppError {
    match error {
        OciDistributionError::AuthenticationFailure(_)
        | OciDistributionError::ImageManifestNotFoundError(_)
        | OciDistributionError::RegistryError { .. }
        | OciDistributionError::UnauthorizedError { .. } => {
            AppError::BadRequest("Image could not be resolved from the registry".into())
        }
        OciDistributionError::ServerError { code, .. } if code < 500 => {
            AppError::BadRequest("Image could not be resolved from the registry".into())
        }
        OciDistributionError::GenericError(Some(message))
            if message.contains("registry") || message.contains("Registry") =>
        {
            AppError::BadRequest(message)
        }
        _ => AppError::ServiceUnavailable("Container registry is unavailable".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn accepts_an_existing_digest_without_network_access() {
        let digest = format!("sha256:{}", "a".repeat(64));
        let result = resolve_image(&format!("nginx@{digest}"), Uuid::nil(), None)
            .await
            .unwrap();
        assert_eq!(result, format!("docker.io/library/nginx@{digest}"));
    }

    #[test]
    fn rejects_a_foreign_internal_repository() {
        let result = internal_repository_name("other/image", "acme");
        assert!(
            matches!(result, Err(AppError::BadRequest(message)) if message.contains("belong to this organization"))
        );
    }

    #[test]
    fn parses_a_project_scoped_internal_repository() {
        assert_eq!(
            internal_repository_name("acme/backend/team/image", "acme").unwrap(),
            ("backend", "team/image")
        );
    }

    #[tokio::test]
    async fn rejects_a_mismatched_external_registry_for_a_digest() {
        let digest = format!("sha256:{}", "a".repeat(64));
        let registry = external_registry::Model {
            id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            name: "private".into(),
            host: "registry.example.com".into(),
            username: "user".into(),
            created_at: Utc::now().fixed_offset(),
            updated_at: Utc::now().fixed_offset(),
        };
        let result = resolve_image(
            &format!("other.example.com/team/image@{digest}"),
            Uuid::nil(),
            Some(&registry),
        )
        .await;

        assert!(
            matches!(result, Err(AppError::BadRequest(message)) if message.contains("does not match image host"))
        );
    }
}
