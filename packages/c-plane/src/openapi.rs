use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "c-plane API",
        version = "0.1.0",
        description = "Control plane API for managing projects, branches, containers, and databases",
    ),
    paths(
        crate::handlers::projects::create_project,
        crate::handlers::projects::get_project,
        crate::handlers::projects::delete_project,
        crate::handlers::projects::create_branch,
        crate::handlers::projects::update_branch,
        crate::handlers::projects::delete_branch,
        crate::handlers::containers::create_container,
        crate::handlers::containers::list_containers,
        crate::handlers::containers::get_container,
        crate::handlers::containers::update_container,
        crate::handlers::containers::delete_container,
        crate::handlers::stateful_databases::create_database,
        crate::handlers::stateful_databases::delete_database,
        crate::handlers::stateful_databases::create_database_branch,
        crate::handlers::stateful_databases::delete_database_branch,
        crate::handlers::serverless_databases::create_database,
        crate::handlers::serverless_databases::delete_database,
        crate::handlers::serverless_databases::create_database_branch,
        crate::handlers::serverless_databases::delete_database_branch,
    ),
    components(
        schemas(
            crate::handlers::projects::CreateProjectRequest,
            crate::handlers::projects::ProjectResponse,
            crate::handlers::projects::CreateBranchRequest,
            crate::handlers::projects::UpdateBranchRequest,
            crate::handlers::projects::BranchResponse,
            crate::handlers::containers::CreateContainerRequest,
            crate::handlers::containers::UpdateContainerRequest,
            crate::handlers::containers::ContainerResponse,
            crate::handlers::containers::ContainerVersionResponse,
            crate::handlers::databases::CreateDatabaseRequest,
            crate::handlers::databases::UpdateDatabaseRequest,
            crate::handlers::databases::DatabaseResponse,
            crate::handlers::databases::CreateDatabaseBranchRequest,
            crate::handlers::databases::DatabaseBranchResponse,
        ),
    ),
    tags(
        (name = "projects", description = "Project management"),
        (name = "branches", description = "Branch management"),
        (name = "containers", description = "Container management"),
        (name = "databases/stateful", description = "Stateful database management"),
        (name = "databases/serverless", description = "Serverless database management"),
    ),
)]
pub struct ApiDoc;
