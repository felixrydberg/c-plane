use axum::{
    Router, middleware,
    routing::{get, post},
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::containers;
use crate::handlers::events;
use crate::handlers::external_registries;
use crate::handlers::health::health_check;
use crate::handlers::internal_s3;
use crate::handlers::internal_secrets;
use crate::handlers::managed_registry;
use crate::handlers::postgres_databases;
use crate::handlers::projects;
use crate::handlers::regions;
use crate::handlers::registry;
use crate::handlers::registry_access_tokens;
use crate::handlers::registry_repositories;
use crate::handlers::registry_tags;
use crate::handlers::storage_access_tokens;
use crate::handlers::storage_buckets;
use crate::handlers::storage_objects;
use crate::middleware::internal_auth;
use crate::middleware::scoped::{self, Role, ScopedRouter};
use crate::openapi::ApiDoc;

// Roles: Member everywhere; stricter requirements are spelled out inline.
pub fn create_routes() -> Router {
    let internal = Router::new()
        .route(
            "/organizations/{organization_id}/transit-key",
            post(internal_secrets::provision_tenant_key),
        )
        .route(
            "/s3-access-tokens/resolve/{access_key}",
            get(internal_s3::resolve_access_token),
        )
        .route(
            "/s3-providers/{provider_id}/credentials",
            get(internal_s3::provider_credentials),
        )
        .route(
            "/organizations/{organization_id}/registry",
            get(managed_registry::resolve_registry),
        )
        .layer(middleware::from_fn(internal_auth::authorize));

    Router::new()
        .nest("/internal", internal)
        // Unauthenticated / non-AuthContext routes: no scope needed.
        .route("/health", get(health_check))
        .route("/api/registry/token", get(registry::issue_token))
        .scoped_route(
            "/api/organization/{organization_id}/regions",
            [scoped::get(
                regions::list_regions,
                "region:read",
                Role::Member,
            )],
        )
        .scoped_route(
            "/api/organization/{organization_id}/environments",
            [scoped::get(
                projects::list_organization_environments,
                "project:read",
                Role::Member,
            )],
        )
        .scoped_route(
            "/api/organization/{organization_id}/projects",
            [
                scoped::get(projects::list_projects, "project:read", Role::Member),
                scoped::post(projects::create_project, "project:create", Role::Member),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/projects/{project_id}",
            [
                scoped::get(projects::get_project, "project:read", Role::Member),
                scoped::delete(projects::delete_project, "project:delete", Role::Owner),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/projects/{project_id}/environments",
            [
                scoped::get(projects::list_environments, "project:read", Role::Member),
                scoped::post(
                    projects::create_environment,
                    "project:manage",
                    Role::Member,
                ),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}",
            [
                scoped::patch(
                    projects::update_environment,
                    "project:manage",
                    Role::Member,
                ),
                scoped::delete(
                    projects::delete_environment,
                    "project:manage",
                    Role::Owner,
                ),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/projects/{project_id}/timelines",
            [scoped::get(
                projects::list_project_timelines,
                "timeline:read",
                Role::Member,
            )],
        )
        .scoped_route(
            "/api/organization/{organization_id}/projects/{project_id}/timelines/{timeline_id}",
            [scoped::get(projects::get_timeline, "timeline:read", Role::Member)],
        )
        .scoped_route(
            "/api/organization/{organization_id}/events",
            [scoped::get(events::list_events, "event:read", Role::Member)],
        )
        .scoped_route(
            "/api/organization/{organization_id}/containers",
            [
                scoped::get(containers::list_containers, "container:read", Role::Member),
                scoped::post(containers::create_container, "container:create", Role::Member),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/containers/{container_id}",
            [
                scoped::get(containers::get_container, "container:read", Role::Member),
                scoped::patch(containers::update_container, "container:update", Role::Member),
                scoped::delete(containers::delete_container, "container:delete", Role::Admin),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/containers/{container_id}/history",
            [scoped::get(
                containers::history::get_container_history,
                "container:read",
                Role::Member,
            )],
        )
        .scoped_route(
            "/api/organization/{organization_id}/containers/{container_id}/deploy",
            [scoped::post(
                containers::redeploy_container,
                "container:update",
                Role::Member,
            )],
        )
        .scoped_route(
            "/api/organization/{organization_id}/databases/postgres",
            [
                scoped::get(postgres_databases::list_databases, "database:postgres:read", Role::Member),
                scoped::post(postgres_databases::create_database, "database:postgres:create", Role::Member),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/databases/postgres/{database_id}",
            [
                scoped::get(postgres_databases::get_database, "database:postgres:read", Role::Member),
                scoped::patch(postgres_databases::update_database, "database:postgres:update", Role::Member),
                scoped::delete(postgres_databases::delete_database, "database:postgres:delete", Role::Admin),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/databases/postgres/{database_id}/branches",
            [
                scoped::get(postgres_databases::list_database_branches, "database:postgres:read", Role::Member),
                scoped::post(postgres_databases::create_database_branch, "database:postgres:manage", Role::Member),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}",
            [
                scoped::patch(postgres_databases::update_database_branch, "database:postgres:manage", Role::Member),
                scoped::delete(postgres_databases::delete_database_branch, "database:postgres:delete", Role::Admin),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens",
            [
                scoped::get(storage_access_tokens::list_access_tokens, "access-token:read", Role::Member),
                scoped::post(storage_access_tokens::create_access_token, "access-token:create", Role::Member),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
            [
                scoped::get(storage_access_tokens::get_access_token, "access-token:read", Role::Member),
                scoped::patch(storage_access_tokens::update_access_token, "access-token:update", Role::Member),
                scoped::delete(storage_access_tokens::revoke_access_token, "access-token:delete", Role::Admin),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/storage/buckets",
            [
                scoped::get(storage_buckets::list_buckets, "bucket:read", Role::Member),
                scoped::post(storage_buckets::create_bucket, "bucket:create", Role::Member),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/storage/buckets/{bucket_id}",
            [scoped::delete(storage_buckets::delete_bucket, "bucket:delete", Role::Admin)],
        )
        .scoped_route(
            "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects",
            [
                scoped::get(storage_objects::list_objects, "bucket:read", Role::Member),
                scoped::delete(storage_objects::delete_objects, "bucket:delete", Role::Admin),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects/download",
            [scoped::get(storage_objects::download_object, "bucket:read", Role::Member)],
        )
        .scoped_route(
            "/api/organization/{organization_id}/registry",
            [
                scoped::get(managed_registry::get_registry, "registry:read", Role::Member),
                scoped::put(managed_registry::activate_registry, "registry:create", Role::Admin),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/registry/garbage-collection",
            [
                scoped::get(
                    managed_registry::get_garbage_collection,
                    "registry:read",
                    Role::Member,
                ),
                scoped::post(
                    managed_registry::run_garbage_collection,
                    "registry:update",
                    Role::Admin,
                ),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/projects/{project_id}/registry/repositories",
            [
                scoped::get(registry_repositories::list_repositories, "registry:read", Role::Member),
                scoped::post(registry_repositories::create_repository, "registry:create", Role::Member),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/projects/{project_id}/registry/repositories/{repository_id}",
            [scoped::delete(registry_repositories::delete_repository, "registry:delete", Role::Admin)],
        )
        .scoped_route(
            "/api/organization/{organization_id}/projects/{project_id}/registry/repositories/{repository_id}/tags",
            [scoped::get(registry_tags::list_tags, "registry:read", Role::Member)],
        )
        .scoped_route(
            "/api/organization/{organization_id}/projects/{project_id}/registry/repositories/{repository_id}/tags/{tag}",
            [scoped::delete(registry_tags::delete_tag, "registry:delete", Role::Admin)],
        )
        .scoped_route(
            "/api/organization/{organization_id}/registry/external-registries",
            [
                scoped::get(external_registries::list_external_registries, "registry:read", Role::Member),
                scoped::post(external_registries::create_external_registry, "registry:create", Role::Member),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/registry/external-registries/{registry_id}",
            [
                scoped::patch(external_registries::rename_external_registry, "registry:update", Role::Member),
                scoped::delete(external_registries::delete_external_registry, "registry:delete", Role::Admin),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/registry/external-registries/{registry_id}/rotate-token",
            [scoped::post(
                external_registries::rotate_external_registry_token,
                "registry:update",
                Role::Member,
            )],
        )
        .scoped_route(
            "/api/organization/{organization_id}/projects/{project_id}/registry/access-tokens",
            [
                scoped::get(registry_access_tokens::list_access_tokens, "access-token:read", Role::Member),
                scoped::post(registry_access_tokens::create_access_token, "access-token:create", Role::Member),
            ],
        )
        .scoped_route(
            "/api/organization/{organization_id}/projects/{project_id}/registry/access-tokens/{token_id}",
            [
                scoped::get(registry_access_tokens::get_access_token, "access-token:read", Role::Member),
                scoped::patch(registry_access_tokens::update_access_token, "access-token:update", Role::Member),
                scoped::delete(registry_access_tokens::revoke_access_token, "access-token:delete", Role::Admin),
            ],
        )
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
}

#[cfg(test)]
mod tests {
    use super::create_routes;
    use crate::middleware::scoped::{Role, registered_min_role, registered_scope};

    /// Every guarded (method, path) must declare exactly the scope it had
    /// before scopes moved onto route declarations.
    #[test]
    fn every_protected_route_has_its_exact_scope() {
        let expected = [
            (
                "GET",
                "/api/organization/{organization_id}/regions",
                "region:read",
                Role::Member,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects",
                "project:read",
                Role::Member,
            ),
            (
                "POST",
                "/api/organization/{organization_id}/projects",
                "project:create",
                Role::Member,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}",
                "project:read",
                Role::Member,
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/projects/{project_id}",
                "project:delete",
                Role::Owner,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/environments",
                "project:read",
                Role::Member,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}/environments",
                "project:read",
                Role::Member,
            ),
            (
                "POST",
                "/api/organization/{organization_id}/projects/{project_id}/environments",
                "project:manage",
                Role::Member,
            ),
            (
                "PATCH",
                "/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}",
                "project:manage",
                Role::Member,
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/projects/{project_id}/environments/{environment_id}",
                "project:manage",
                Role::Owner,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}/timelines",
                "timeline:read",
                Role::Member,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}/timelines/{timeline_id}",
                "timeline:read",
                Role::Member,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/events",
                "event:read",
                Role::Member,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/containers",
                "container:read",
                Role::Member,
            ),
            (
                "POST",
                "/api/organization/{organization_id}/containers",
                "container:create",
                Role::Member,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/containers/{container_id}",
                "container:read",
                Role::Member,
            ),
            (
                "PATCH",
                "/api/organization/{organization_id}/containers/{container_id}",
                "container:update",
                Role::Member,
            ),
            (
                "POST",
                "/api/organization/{organization_id}/containers/{container_id}/deploy",
                "container:update",
                Role::Member,
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/containers/{container_id}",
                "container:delete",
                Role::Admin,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/databases/postgres",
                "database:postgres:read",
                Role::Member,
            ),
            (
                "POST",
                "/api/organization/{organization_id}/databases/postgres",
                "database:postgres:create",
                Role::Member,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/databases/postgres/{database_id}",
                "database:postgres:read",
                Role::Member,
            ),
            (
                "PATCH",
                "/api/organization/{organization_id}/databases/postgres/{database_id}",
                "database:postgres:update",
                Role::Member,
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/databases/postgres/{database_id}",
                "database:postgres:delete",
                Role::Admin,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/databases/postgres/{database_id}/branches",
                "database:postgres:read",
                Role::Member,
            ),
            (
                "POST",
                "/api/organization/{organization_id}/databases/postgres/{database_id}/branches",
                "database:postgres:manage",
                Role::Member,
            ),
            (
                "PATCH",
                "/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}",
                "database:postgres:manage",
                Role::Member,
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}",
                "database:postgres:delete",
                Role::Admin,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens",
                "access-token:read",
                Role::Member,
            ),
            (
                "POST",
                "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens",
                "access-token:create",
                Role::Member,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
                "access-token:read",
                Role::Member,
            ),
            (
                "PATCH",
                "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
                "access-token:update",
                Role::Member,
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/projects/{project_id}/storage/access-tokens/{token_id}",
                "access-token:delete",
                Role::Admin,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/storage/buckets",
                "bucket:read",
                Role::Member,
            ),
            (
                "POST",
                "/api/organization/{organization_id}/storage/buckets",
                "bucket:create",
                Role::Member,
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/storage/buckets/{bucket_id}",
                "bucket:delete",
                Role::Admin,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects",
                "bucket:read",
                Role::Member,
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects",
                "bucket:delete",
                Role::Admin,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/storage/buckets/{bucket_id}/objects/download",
                "bucket:read",
                Role::Member,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/registry",
                "registry:read",
                Role::Member,
            ),
            (
                "PUT",
                "/api/organization/{organization_id}/registry",
                "registry:create",
                Role::Admin,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/registry/garbage-collection",
                "registry:read",
                Role::Member,
            ),
            (
                "POST",
                "/api/organization/{organization_id}/registry/garbage-collection",
                "registry:update",
                Role::Admin,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}/registry/repositories",
                "registry:read",
                Role::Member,
            ),
            (
                "POST",
                "/api/organization/{organization_id}/projects/{project_id}/registry/repositories",
                "registry:create",
                Role::Member,
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/projects/{project_id}/registry/repositories/{repository_id}",
                "registry:delete",
                Role::Admin,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}/registry/repositories/{repository_id}/tags",
                "registry:read",
                Role::Member,
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/projects/{project_id}/registry/repositories/{repository_id}/tags/{tag}",
                "registry:delete",
                Role::Admin,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/registry/external-registries",
                "registry:read",
                Role::Member,
            ),
            (
                "POST",
                "/api/organization/{organization_id}/registry/external-registries",
                "registry:create",
                Role::Member,
            ),
            (
                "PATCH",
                "/api/organization/{organization_id}/registry/external-registries/{registry_id}",
                "registry:update",
                Role::Member,
            ),
            (
                "POST",
                "/api/organization/{organization_id}/registry/external-registries/{registry_id}/rotate-token",
                "registry:update",
                Role::Member,
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/registry/external-registries/{registry_id}",
                "registry:delete",
                Role::Admin,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}/registry/access-tokens",
                "access-token:read",
                Role::Member,
            ),
            (
                "POST",
                "/api/organization/{organization_id}/projects/{project_id}/registry/access-tokens",
                "access-token:create",
                Role::Member,
            ),
            (
                "GET",
                "/api/organization/{organization_id}/projects/{project_id}/registry/access-tokens/{token_id}",
                "access-token:read",
                Role::Member,
            ),
            (
                "PATCH",
                "/api/organization/{organization_id}/projects/{project_id}/registry/access-tokens/{token_id}",
                "access-token:update",
                Role::Member,
            ),
            (
                "DELETE",
                "/api/organization/{organization_id}/projects/{project_id}/registry/access-tokens/{token_id}",
                "access-token:delete",
                Role::Admin,
            ),
        ];

        let _ = create_routes();

        for (method, path, scope, min_role) in expected {
            assert_eq!(
                registered_scope(method, path),
                Some(scope),
                "{method} {path}"
            );
            assert_eq!(
                registered_min_role(method, path),
                Some(min_role),
                "{method} {path} minimum role"
            );
        }

        // HEAD reuses GET's scope; unguarded public routes stay unlisted.
        assert_eq!(
            registered_scope("HEAD", "/api/organization/{organization_id}/regions"),
            Some("region:read")
        );
        assert_eq!(
            registered_min_role("HEAD", "/api/organization/{organization_id}/regions"),
            Some(Role::Member)
        );
        assert_eq!(registered_scope("GET", "/health"), None);
        assert_eq!(registered_scope("GET", "/api/registry/token"), None);
    }
}
