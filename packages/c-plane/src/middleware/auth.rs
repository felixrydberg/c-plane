use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use ory_client::apis::{configuration::Configuration, frontend_api};
use uuid::Uuid;

pub async fn auth_middleware(mut request: Request, next: Next) -> Result<Response, StatusCode> {
    let cookie = request
        .headers()
        .get("cookie")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let kratos_public_url = std::env::var("KRATOS_PUBLIC_URL")
        .unwrap_or_else(|_| "http://kratos:4433".to_string())
        .trim_end_matches('/')
        .to_string();

    let mut ory_config = Configuration::new();
    ory_config.base_path = kratos_public_url;

    let session = frontend_api::to_session(&ory_config, None, Some(cookie), None)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let identity_id = session
        .identity
        .as_ref()
        .map(|identity| identity.id.as_str())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let user_id = Uuid::parse_str(identity_id).map_err(|_| StatusCode::UNAUTHORIZED)?;

    request.extensions_mut().insert(user_id);
    Ok(next.run(request).await)
}
