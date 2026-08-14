use utoipa::openapi::{
    extensions::Extensions,
    path::Operation,
    security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
};
use utoipa::{Modify, OpenApi};

use crate::middleware::auth::required_scope;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "apiKey",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                "x-api-key",
                "API key with the operation's required scope.",
            ))),
        );
        components.add_security_scheme(
            "registryBasic",
            SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Basic).build()),
        );
        components.add_security_scheme(
            "serviceToken",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                "x-cplane-token",
                "Internal service token.",
            ))),
        );

        for (path, item) in &mut openapi.paths.paths {
            document_scope("GET", path, item.get.as_mut());
            document_scope("POST", path, item.post.as_mut());
            document_scope("PATCH", path, item.patch.as_mut());
            document_scope("DELETE", path, item.delete.as_mut());
        }
    }
}

fn document_scope(method: &str, path: &str, operation: Option<&mut Operation>) {
    let Some(operation) = operation else {
        return;
    };

    if let Some(scope) = required_scope(if method == "HEAD" { "GET" } else { method }, path) {
        operation.security = Some(vec![utoipa::openapi::security::SecurityRequirement::new(
            "apiKey",
            Vec::<String>::new(),
        )]);
        operation
            .extensions
            .get_or_insert_with(Default::default)
            .insert("x-cplane-required-scope".into(), serde_json::json!(scope));
    } else if operation.operation_id.as_deref() == Some("issue_token") {
        operation.security = Some(vec![utoipa::openapi::security::SecurityRequirement::new(
            "registryBasic",
            Vec::<String>::new(),
        )]);
        operation.extensions = Some(Extensions::from_iter([(
            "x-cplane-required-scope",
            serde_json::json!(["registry:pull", "registry:push"]),
        )]));
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "c-plane API",
        version = "0.1.0",
        description = "Control plane API for managing projects, environments, containers, and databases",
    ),
    paths(
        crate::handlers::projects::create_project,
        crate::handlers::projects::list_projects,
        crate::handlers::projects::get_project,
        crate::handlers::projects::delete_project,
        crate::handlers::projects::list_organization_environments,
        crate::handlers::projects::list_environments,
        crate::handlers::projects::create_environment,
        crate::handlers::projects::update_environment,
        crate::handlers::projects::delete_environment,
        crate::handlers::projects::list_project_timelines,
        crate::handlers::projects::get_timeline,
        crate::handlers::events::list_events,
        crate::handlers::health::health_check,
        crate::handlers::regions::list_regions,
        crate::handlers::containers::create_container,
        crate::handlers::containers::list_containers,
        crate::handlers::containers::get_container,
        crate::handlers::containers::update_container,
        crate::handlers::containers::delete_container,
        crate::handlers::postgres_databases::create_database,
        crate::handlers::postgres_databases::list_databases,
        crate::handlers::postgres_databases::get_database,
        crate::handlers::postgres_databases::update_database,
        crate::handlers::postgres_databases::delete_database,
        crate::handlers::postgres_databases::list_database_branches,
        crate::handlers::postgres_databases::update_database_branch,
        crate::handlers::postgres_databases::create_database_branch,
        crate::handlers::postgres_databases::delete_database_branch,
        crate::handlers::storage_buckets::create_bucket,
        crate::handlers::storage_buckets::delete_bucket,
        crate::handlers::storage_buckets::list_buckets,
        crate::handlers::storage_access_tokens::create_access_token,
        crate::handlers::storage_access_tokens::get_access_token,
        crate::handlers::storage_access_tokens::list_access_tokens,
        crate::handlers::storage_access_tokens::revoke_access_token,
        crate::handlers::storage_access_tokens::update_access_token,
        crate::handlers::registry::issue_token,
        crate::handlers::registry::maintenance_status,
        crate::handlers::registry_access_tokens::create_access_token,
        crate::handlers::registry_access_tokens::get_access_token,
        crate::handlers::registry_access_tokens::list_access_tokens,
        crate::handlers::registry_access_tokens::revoke_access_token,
        crate::handlers::registry_access_tokens::update_access_token,
        crate::handlers::registry_repositories::create_repository,
        crate::handlers::registry_repositories::list_repositories,
        crate::handlers::registry_repositories::delete_repository,
        crate::handlers::external_registries::list_external_registries,
        crate::handlers::external_registries::create_external_registry,
        crate::handlers::external_registries::rename_external_registry,
        crate::handlers::external_registries::rotate_external_registry_token,
        crate::handlers::external_registries::delete_external_registry,
        crate::handlers::external_registries::delete_secret_internal,
        crate::handlers::internal_s3::resolve_access_token,
        crate::handlers::internal_s3::provider_credentials,
    ),
    components(
        schemas(
            crate::errors::ErrorResponse,
            crate::handlers::projects::CreateProjectRequest,
            crate::handlers::projects::ProjectResponse,
            crate::handlers::projects::CreateEnvironmentRequest,
            crate::handlers::projects::UpdateEnvironmentRequest,
            crate::handlers::projects::EnvironmentResponse,
            crate::handlers::projects::EnvironmentWithProjectResponse,
            crate::handlers::projects::TimelineResponse,
            crate::handlers::projects::ResolvedContainerPin,
            crate::handlers::projects::ResolvedTimelineResponse,
            crate::handlers::health::HealthResponse,
            crate::handlers::regions::RegionResponse,
            crate::handlers::events::EventResponse,
            crate::handlers::containers::CreateContainerRequest,
            crate::handlers::containers::UpdateContainerRequest,
            crate::handlers::containers::ContainerResponse,
            crate::handlers::containers::ContainerVersionResponse,
            crate::handlers::databases::CreateDatabaseRequest,
            crate::handlers::databases::UpdateDatabaseRequest,
            crate::handlers::databases::DatabaseResponse,
            crate::handlers::databases::CreateDatabaseBranchRequest,
            crate::handlers::databases::UpdateDatabaseBranchRequest,
            crate::handlers::databases::DatabaseBranchResponse,
            crate::utils::pagination::PaginatedResponse<crate::handlers::projects::ProjectResponse>,
            crate::utils::pagination::PaginationMeta,
            crate::handlers::storage_buckets::CreateBucketRequest,
            crate::handlers::storage_buckets::BucketResponse,
            crate::handlers::storage_access_tokens::CreateAccessTokenRequest,
            crate::handlers::storage_access_tokens::UpdateAccessTokenRequest,
            crate::handlers::storage_access_tokens::BucketPermissionRequest,
            crate::handlers::storage_access_tokens::AccessTokenResponse,
            crate::handlers::storage_access_tokens::AccessTokenDetailsResponse,
            crate::handlers::storage_access_tokens::CreatedAccessTokenResponse,
            crate::handlers::registry::RegistryTokenResponse,
            crate::handlers::registry::RegistryMaintenanceResponse,
            crate::handlers::registry_access_tokens::CreateRegistryAccessTokenRequest,
            crate::handlers::registry_access_tokens::UpdateRegistryAccessTokenRequest,
            crate::handlers::registry_access_tokens::RepositoryPermissionRequest,
            crate::handlers::registry_access_tokens::RegistryAccessTokenResponse,
            crate::handlers::registry_access_tokens::RegistryAccessTokenDetailsResponse,
            crate::handlers::registry_access_tokens::CreatedRegistryAccessTokenResponse,
            crate::handlers::registry_repositories::CreateRegistryRepositoryRequest,
            crate::handlers::registry_repositories::RegistryRepositoryResponse,
            crate::handlers::external_registries::CreateExternalRegistryRequest,
            crate::handlers::external_registries::RenameExternalRegistryRequest,
            crate::handlers::external_registries::RotateExternalRegistryTokenRequest,
            crate::handlers::external_registries::ExternalRegistryResponse,
            crate::handlers::internal_s3::ResolvedS3AccessToken,
            crate::handlers::internal_s3::ResolvedS3BucketPermission,
            crate::services::s3_providers::S3ProviderCredentials,
        ),
    ),
    tags(
        (name = "projects", description = "Project management"),
        (name = "environments", description = "Environment management"),
        (name = "containers", description = "Container management"),
        (name = "events", description = "Organization activity"),
        (name = "databases/postgres", description = "Postgres database management"),
        (name = "storage", description = "S3 bucket and access token management"),
        (name = "registry", description = "OCI registry authentication"),
        (name = "internal", description = "Internal service endpoints"),
    ),
    modifiers(&SecurityAddon),
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_reuses_get_scope() {
        let mut operation = Operation::default();

        document_scope(
            "HEAD",
            "/api/organization/{organization_id}/regions",
            Some(&mut operation),
        );

        assert_eq!(
            operation.extensions.unwrap().get("x-cplane-required-scope"),
            Some(&serde_json::json!("region:read"))
        );
    }
}
