use axum::{
    Router,
    routing::{get, patch, post},
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::containers;
use crate::handlers::events;
use crate::handlers::health::health_check;
use crate::handlers::projects;
use crate::handlers::serverless_databases;
use crate::handlers::stateful_databases;
use crate::openapi::ApiDoc;

pub fn create_routes() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route(
            "/api/organization/{organization_id}/branches",
            get(projects::list_organization_branches),
        )
        .route(
            "/api/organization/{organization_id}/projects",
            get(projects::list_projects).post(projects::create_project),
        )
        .route(
            "/api/organization/{organization_id}/projects/{project_id}",
            get(projects::get_project).delete(projects::delete_project),
        )
        .route(
            "/api/organization/{organization_id}/projects/{project_id}/branches",
            get(projects::list_branches).post(projects::create_branch),
        )
        .route(
            "/api/organization/{organization_id}/projects/{project_id}/branches/{branch_id}",
            patch(projects::update_branch).delete(projects::delete_branch),
        )
        .route(
            "/api/organization/{organization_id}/projects/{project_id}/timelines",
            get(projects::list_project_timelines),
        )
        .route(
            "/api/organization/{organization_id}/events",
            get(events::list_events),
        )
        .route(
            "/api/organization/{organization_id}/projects/{project_id}/timelines/{timeline_id}",
            get(projects::get_timeline),
        )
        .route(
            "/api/organization/{organization_id}/containers",
            get(containers::list_containers).post(containers::create_container),
        )
        .route(
            "/api/organization/{organization_id}/containers/{container_id}",
            get(containers::get_container)
                .patch(containers::update_container)
                .delete(containers::delete_container),
        )
        .route(
            "/api/organization/{organization_id}/databases/stateful",
            get(stateful_databases::list_databases).post(stateful_databases::create_database),
        )
        .route(
            "/api/organization/{organization_id}/databases/stateful/{database_id}",
            get(stateful_databases::get_database)
                .patch(stateful_databases::update_database)
                .delete(stateful_databases::delete_database),
        )
        .route(
            "/api/organization/{organization_id}/databases/serverless",
            get(serverless_databases::list_databases).post(serverless_databases::create_database),
        )
        .route(
            "/api/organization/{organization_id}/databases/serverless/{database_id}",
            get(serverless_databases::get_database)
                .patch(serverless_databases::update_database)
                .delete(serverless_databases::delete_database),
        )
        .route(
            "/api/organization/{organization_id}/databases/stateful/{database_id}/branches",
            post(stateful_databases::create_database_branch),
        )
        .route(
            "/api/organization/{organization_id}/databases/stateful/{database_id}/branches/{branch_id}",
            patch(stateful_databases::update_database_branch)
            .delete(stateful_databases::delete_database_branch),
        )
        .route(
            "/api/organization/{organization_id}/databases/serverless/{database_id}/branches",
            post(serverless_databases::create_database_branch),
        )
        .route(
            "/api/organization/{organization_id}/databases/serverless/{database_id}/branches/{branch_id}",
            patch(serverless_databases::update_database_branch)
            .delete(serverless_databases::delete_database_branch),
        )
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
}
