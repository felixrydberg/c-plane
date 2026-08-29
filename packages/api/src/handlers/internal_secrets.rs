use axum::{extract::Path, http::StatusCode};
use uuid::Uuid;

use crate::{errors::AppError, state::get_app_state};

#[utoipa::path(
    post,
    path = "/internal/organizations/{organization_id}/transit-key",
    params(("organization_id" = Uuid, Path)),
    responses(
        (status = 204, description = "Tenant Transit key provisioned"),
        (status = 401, body = crate::errors::ErrorResponse),
        (status = 500, body = crate::errors::ErrorResponse),
    ),
    security(("serviceToken" = [])),
    tag = "internal",
)]
pub async fn provision_tenant_key(
    Path(organization_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let state = get_app_state();
    lib::secrets::create_key(
        &state.secrets,
        &format!("tenant-{}", organization_id.simple()),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
