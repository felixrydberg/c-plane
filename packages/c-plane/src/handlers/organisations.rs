use axum::{
    Json,
    extract::{Extension, Path},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::entities::{OrganisationMemberModel, OrganisationModel};
use crate::services::organisations::{CreateOrganisationData, create_organisation, get_organisation};
use crate::state::get_app_state;

#[derive(Debug, Deserialize)]
pub struct CreateOrganisationRequest {
    pub name: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateOrganisationResponse {
    pub organisation: OrganisationModel,
    pub organisation_member: OrganisationMemberModel,
}

pub async fn create_organisation_handler(
    Json(request): Json<CreateOrganisationRequest>,
    Extension(user_id): Extension<Uuid>,
) -> Result<(StatusCode, Json<CreateOrganisationResponse>), AppError> {
    let state = get_app_state();

    let (organisation, organisation_member) = create_organisation(
        state.db,
        CreateOrganisationData {
            identity_id: user_id,
            name: request.name,
            description: request.description,
        },
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(CreateOrganisationResponse {
            organisation,
            organisation_member,
        }),
    ))
}

pub async fn get_organisation_handler(
    Path(organisation_id): Path<Uuid>,
) -> Result<Json<OrganisationModel>, AppError> {
    let state = get_app_state();
    let organisation = get_organisation(state.db, organisation_id).await?;
    Ok(Json(organisation))
}
