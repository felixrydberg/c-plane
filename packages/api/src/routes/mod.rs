use axum::{
    Router, middleware,
    routing::{delete, get, patch, post},
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::containers;
use crate::handlers::events;
use crate::handlers::external_registries;
use crate::handlers::health::health_check;
use crate::handlers::internal_s3;
use crate::handlers::postgres_databases;
use crate::handlers::projects;
use crate::handlers::regions;
use crate::handlers::registry;
use crate::handlers::registry_access_tokens;
use crate::handlers::registry_repositories;
use crate::handlers::storage_access_tokens;
use crate::handlers::storage_buckets;
use crate::middleware::internal_auth;
use crate::openapi::ApiDoc;

pub fn create_routes() -> Router {
    let internal = Router::new()
        .route(
            "/s3-access-tokens/resolve/{access_key}",
            get(internal_s3::resolve_access_token),
        )
        .route(
            "/s3-providers/{provider_id}/credentials",
            get(internal_s3::provider_credentials),
        )
        .route(
            "/organizations/{organization_id}/external-registries/{registry_id}/secret",
            delete(external_registries::delete_secret_internal),
        )
        .layer(middleware::from_fn(internal_auth::authorize));

    Router::new()
        .nest("/internal", internal)
        .route("/health", get(health_check))
        .route("/api/registry/token", get(registry::issue_token))
        .route(
            "/api/organization/{organization_id}/registry/maintenance",
            get(registry::maintenance_status),
        )
        .route(
            "/api/organization/{organization_id}/registry/repositories",
            get(registry_repositories::list_repositories)
                .post(registry_repositories::create_repository),
        )
        .route(
            "/api/organization/{organization_id}/registry/repositories/{repository_id}",
            delete(registry_repositories::delete_repository),
        )
        .route(
            "/api/organization/{organization_id}/registry/external-registries",
            get(external_registries::list_external_registries)
                .post(external_registries::create_external_registry),
        )
        .route(
            "/api/organization/{organization_id}/registry/external-registries/{registry_id}",
            patch(external_registries::rename_external_registry)
                .delete(external_registries::delete_external_registry),
        )
        .route(
            "/api/organization/{organization_id}/registry/external-registries/{registry_id}/rotate-token",
            axum::routing::post(external_registries::rotate_external_registry_token),
        )
        .route(
            "/api/organization/{organization_id}/registry/access-tokens",
            get(registry_access_tokens::list_access_tokens)
                .post(registry_access_tokens::create_access_token),
        )
        .route(
            "/api/organization/{organization_id}/registry/access-tokens/{token_id}",
            get(registry_access_tokens::get_access_token)
                .patch(registry_access_tokens::update_access_token)
                .delete(registry_access_tokens::revoke_access_token),
        )
        .route(
            "/api/organization/{organization_id}/regions",
            get(regions::list_regions),
        )
        .route(
            "/api/organization/{organization_id}/environments",
            get(projects::list_organization_environments),
        )
        .route(
            "/api/organization/{organization_id}/projects",
            get(projects::list_projects)
                .post(projects::create_project),
        )
        .route(
            "/api/organization/{organization_id}/projects/{project_id}",
            get(projects::get_project)
                .delete(projects::delete_project),
        )
        .route(
            "/api/organization/{organization_id}/projects/{project_id}/environments",
            get(projects::list_environments)
                .post(projects::create_environment),
        )
        .route(
            "/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}",
            patch(projects::update_environment).delete(projects::delete_environment),
        )
        .route(
            "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens",
            get(storage_access_tokens::list_access_tokens)
                .post(storage_access_tokens::create_access_token),
        )
        .route(
            "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
            get(storage_access_tokens::get_access_token)
                .patch(storage_access_tokens::update_access_token)
                .delete(storage_access_tokens::revoke_access_token),
        )
        .route(
            "/api/organization/{organization_id}/storage/buckets/{bucket_id}",
            delete(storage_buckets::delete_bucket),
        )
        .route(
            "/api/organization/{organization_id}/storage/buckets",
            get(storage_buckets::list_buckets)
                .post(storage_buckets::create_bucket),
        )
        .route(
            "/api/organization/{organization_id}/projects/{project_id}/timelines",
            get(projects::list_project_timelines),
        )
        .route(
            "/api/organization/{organization_id}/projects/{project_id}/timelines/{timeline_id}",
            get(projects::get_timeline),
        )
        .route(
            "/api/organization/{organization_id}/events",
            get(events::list_events),
        )
        .route(
            "/api/organization/{organization_id}/containers",
            get(containers::list_containers)
                .post(containers::create_container),
        )
        .route(
            "/api/organization/{organization_id}/containers/{container_id}",
            get(containers::get_container)
                .patch(containers::update_container)
                .delete(containers::delete_container),
        )
        .route(
            "/api/organization/{organization_id}/containers/{container_id}/deploy",
            post(containers::redeploy_container),
        )
        .route(
            "/api/organization/{organization_id}/databases/postgres",
            get(postgres_databases::list_databases)
                .post(postgres_databases::create_database),
        )
        .route(
            "/api/organization/{organization_id}/databases/postgres/{database_id}",
            get(postgres_databases::get_database)
                .patch(postgres_databases::update_database)
                .delete(postgres_databases::delete_database),
        )
        .route(
            "/api/organization/{organization_id}/databases/postgres/{database_id}/branches",
            get(postgres_databases::list_database_branches)
                .post(postgres_databases::create_database_branch),
        )
        .route(
            "/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}",
            patch(postgres_databases::update_database_branch)
                .delete(postgres_databases::delete_database_branch),
        )
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
}
