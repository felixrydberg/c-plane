use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse, FromRequest,
    body::{EitherBody, BoxBody},
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use uuid::Uuid;

pub struct AuthMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService { service }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let user_id = req
            .headers()
            .get("X-User")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| Uuid::parse_str(s).ok());

        if let Some(id) = user_id {
            req.extensions_mut().insert(id);  // Store Uuid directly
            let fut = self.service.call(req);
            Box::pin(async move { 
                let res = fut.await?;
                Ok(res.map_into_left_body())
            })
        } else {
            let response = HttpResponse::Unauthorized().finish();
            let (req, _) = req.into_parts();
            Box::pin(async move { 
                Ok(ServiceResponse::new(req, response).map_into_right_body()) 
            })
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UserId(pub Uuid);

impl UserId {
    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl FromRequest for UserId {
    type Error = crate::errors::AppError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let result = req
            .extensions()
            .get::<Uuid>()  // Get Uuid directly
            .copied()
            .map(UserId)
            .ok_or_else(|| crate::errors::AppError::Unauthorized("User not authenticated".to_string()));
        
        ready(result)
    }
}
