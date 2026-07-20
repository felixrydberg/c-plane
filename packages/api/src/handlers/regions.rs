use axum::{Json, extract::Path};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    errors::AppError,
    middleware::auth::AuthContext,
    models::entities::region::{self, RegionRoutingMode, RegionStatus},
};

use super::databases::verify_org_access;

#[derive(Serialize, ToSchema)]
pub struct RegionResponse {
    pub id: Uuid,
    pub display_name: String,
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/regions",
    params(("organization_id" = Uuid, Path, description = "Organization ID")),
    responses((status = 200, description = "Available regions", body = Vec<RegionResponse>)),
    tag = "regions",
)]
pub async fn list_regions(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
) -> Result<Json<Vec<RegionResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let regions = region::Entity::find()
        .filter(region::Column::Status.eq(RegionStatus::Active))
        .filter(region::Column::RoutingMode.ne(RegionRoutingMode::Disabled))
        .order_by_asc(region::Column::DisplayName)
        .all(scoped.connection())
        .await?;
    scoped.commit().await?;

    Ok(Json(
        regions
            .into_iter()
            .map(|region| RegionResponse {
                id: region.id,
                display_name: region.display_name,
            })
            .collect(),
    ))
}
