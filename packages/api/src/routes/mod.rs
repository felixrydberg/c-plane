use axum::{
    Router,
    routing::{delete, get, patch},
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::containers;
use crate::handlers::events;
use crate::handlers::health::health_check;
use crate::handlers::postgres_databases;
use crate::handlers::projects;
use crate::handlers::regions;
use crate::handlers::registry;
use crate::handlers::registry_access_tokens;
use crate::handlers::registry_repositories;
use crate::handlers::storage_access_tokens;
use crate::handlers::storage_buckets;
use crate::openapi::ApiDoc;

pub fn create_routes() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/registry/token", get(registry::issue_token))
        .route(
            "/api/organization/{organization_id}/registry/repositories",
            get(registry_repositories::list_repositories)
                .post(registry_repositories::create_repository),
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
            "/api/organization/{organization_id}/branches",
            get(projects::list_organization_branches),
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
            "/api/organization/{organization_id}/projects/{project_id}/branches",
            get(projects::list_branches)
                .post(projects::create_branch),
        )
        .route(
            "/api/organization/{organization_id}/projects/{project_id}/branches/{branch_id}",
            patch(projects::update_branch).delete(projects::delete_branch),
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
