use actix_web::{HttpResponse, Result, get, post, web};
use chrono::NaiveDateTime;
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::{AppError, OrganisationError};
use crate::models::entities::OrganisationMemberModel;
use crate::models::entities::OrganisationModel;
use crate::models::{OrganisationRole, organisation_member};
use crate::services::organisations::{
    CreateOrganisationData, create_organisation, get_organisation,
};
use crate::middleware::auth::{UserId, AuthMiddleware};
use crate::state::get_app_state;

#[derive(Serialize, Deserialize)]
struct OrganisationResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub created_by: Uuid,
}

impl From<OrganisationModel> for OrganisationResponse {
    fn from(organisation: OrganisationModel) -> Self {
        Self {
            id: organisation.id,
            name: organisation.name,
            description: organisation.description,
            avatar_url: organisation.avatar_url,
            is_active: organisation.is_active,
            created_at: organisation.created_at,
            updated_at: organisation.updated_at,
            created_by: organisation.created_by,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct OrganisationMemberResponse {
    pub id: Uuid,
    pub organisation_id: Uuid,
    pub role: OrganisationRole,
    pub is_active: bool,
    pub joined_at: NaiveDateTime,
    pub invitation_accepted_at: NaiveDateTime,
}

impl From<OrganisationMemberModel> for OrganisationMemberResponse {
    fn from(organisation_member: OrganisationMemberModel) -> Self {
        Self {
            id: organisation_member.id,
            organisation_id: organisation_member.organisation_id,
            role: organisation_member.role,
            is_active: organisation_member.is_active,
            joined_at: organisation_member.joined_at,
            invitation_accepted_at: organisation_member.invitation_accepted_at,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct CreateOrganisationRequest {
    pub name: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct CreateOrganisationResponse {
    pub organisation: OrganisationResponse,
    pub organisation_member: OrganisationMemberResponse,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/organisations")
            .wrap(AuthMiddleware)
            .service(create_organisation_handler)
            .service(get_organisation_handler)
    );
}

#[post("/")]
async fn create_organisation_handler(
    request: web::Json<CreateOrganisationRequest>,
    user_id: UserId,
) -> Result<HttpResponse, AppError> {
    let created_by = user_id.into_inner();
    let state = get_app_state();

    let result = create_organisation(
        state.db,
        CreateOrganisationData {
            identity_id: created_by,
            name: request.name.clone(),
            description: request.description.clone(),
            avatar_url: request.avatar_url.clone(),
        },
    )
    .await;

    match result {
        Ok((organisation, organisation_member)) => {
            Ok(HttpResponse::Ok().json(CreateOrganisationResponse {
                organisation: OrganisationResponse::from(organisation),
                organisation_member: OrganisationMemberResponse::from(organisation_member),
            }))
        }
        Err(err) => Err(err),
    }
}

#[get("/{id}")]
pub async fn get_organisation_handler(
    db: web::Data<DatabaseConnection>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    // let organisation_id = path.into_inner();

    // let organisation = Organisation::find_by_id(organisation_id)
    //     .one(&**db)
    //     .await?
    //     .ok_or_else(|| AppError::Organisation(OrganisationError::OrganisationNotFound(organisation_id)))?;

    Ok(HttpResponse::Ok().finish())
}
