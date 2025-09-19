use std::str::FromStr;

use actix_web::{HttpResponse, Result, post, web};
use serde::{Deserialize, Serialize};
use ory_client::models::{Identity};
use uuid::Uuid;

use crate::{errors::AppError, middleware::api::ApiKey, services::organisations::{create_organisation, CreateOrganisationData}, state::get_app_state};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(after_registration_handler);
}

#[derive(Deserialize, Debug)]
struct IdentityTraits {
    name: Name,
    email: String,
}

#[derive(Deserialize, Debug)]
struct Name {
    first: String,
    last: String,
}

#[derive(Deserialize, Debug)]
struct AfterRegistrationRequest {
    flow_id: String,
    identity: Identity,
}

#[derive(Serialize)]
struct WebhookResponse {
    success: bool,
    message: String,
}

#[post("/after-registration")]
async fn after_registration_handler(
    payload: web::Json<AfterRegistrationRequest>,
    api_key: ApiKey
) -> Result<HttpResponse, AppError> {
    let state = get_app_state();
    if api_key.into_inner() != state.config.kratos_api_key {
        return Err(AppError::Unauthorized("Invalid API key".to_string()));
    }

    let traits = payload.identity.traits.as_ref()
        .ok_or_else(|| AppError::Internal("Identity traits not found".to_string()))?;
    let identity_traits: IdentityTraits = serde_json::from_value(traits.clone())
        .map_err(|_| AppError::Internal("Invalid identity traits format".to_string()))?;
    let identity_id = Uuid::from_str(&payload.identity.id)
        .map_err(|_| AppError::Internal("Invalid identity ID format".to_string()))?;
    let data = CreateOrganisationData {
        identity_id,
        name: format!("{} {}", identity_traits.name.first, identity_traits.name.last),
        description: None,
        avatar_url: None,
    };

    let _organisation = create_organisation(state.db, data).await?;
    Ok(HttpResponse::Ok().json(WebhookResponse {
        success: true,
        message: "Organisation created successfully".to_string(),
    }))
  }
