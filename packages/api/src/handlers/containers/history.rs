use super::*;
use lib::services::container_history::{self, ContainerHistoryInput};
pub use lib::services::container_history::{
    ContainerChangeType, ContainerHistoryBaseline, ContainerHistoryChange, ContainerHistoryEntry,
    ContainerHistoryPage,
};

#[derive(Deserialize, ToSchema)]
pub struct ContainerHistoryQuery {
    pub environment_id: Uuid,
    pub timeline_id: Uuid,
    pub limit: Option<u64>,
    pub cursor: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/containers/{container_id}/history",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("container_id" = Uuid, Path, description = "Container ID"),
        ("environment_id" = Uuid, Query, description = "Environment in the container's project"),
        ("timeline_id" = Uuid, Query, description = "Selected revision whose ancestry to show"),
        ("limit" = Option<u64>, Query, description = "History entries per page (default 10, max 50)"),
        ("cursor" = Option<String>, Query, description = "Continuation cursor from a previous page"),
    ),
    responses(
        (status = 200, description = "Container configuration changes, newest first", body = ContainerHistoryPage),
        (status = 404, description = "Container, environment, revision, or manifest not found"),
    ),
    tag = "containers",
)]
pub async fn get_container_history(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, container_id)): Path<(Uuid, Uuid)>,
    axum::extract::Query(query): axum::extract::Query<ContainerHistoryQuery>,
) -> Result<Json<ContainerHistoryPage>, AppError> {
    let page = container_history::get(
        &tenant_db,
        organization_id,
        container_id,
        ContainerHistoryInput {
            environment_id: query.environment_id,
            timeline_id: query.timeline_id,
            limit: query.limit,
            cursor: query.cursor,
        },
    )
    .await?;
    Ok(Json(page))
}
