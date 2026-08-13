use axum::{extract::Request, http::HeaderMap, middleware::Next, response::Response};

use crate::{errors::AppError, state::get_app_state};

pub async fn authorize(request: Request, next: Next) -> Result<Response, AppError> {
    let expected = get_app_state().config.service_token;
    if !valid_service_token(request.headers(), &expected) {
        return Err(AppError::Unauthorized("Invalid service token".into()));
    }
    Ok(next.run(request).await)
}

fn valid_service_token(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get("x-cplane-token")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|supplied| supplied == expected)
}

#[cfg(test)]
mod tests {
    use super::valid_service_token;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn internal_routes_require_the_service_token() {
        let mut headers = HeaderMap::new();
        assert!(!valid_service_token(&headers, "correct"));

        headers.insert("x-cplane-token", HeaderValue::from_static("wrong"));
        assert!(!valid_service_token(&headers, "correct"));

        headers.insert("x-cplane-token", HeaderValue::from_static("correct"));
        assert!(valid_service_token(&headers, "correct"));
    }
}
