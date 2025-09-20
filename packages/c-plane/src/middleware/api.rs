use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse, FromRequest,
    body::{EitherBody, BoxBody},
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};

pub struct ApiMiddleware;

impl<S, B> Transform<S, ServiceRequest> for ApiMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Transform = ApiMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ApiMiddlewareService { service }))
    }
}

pub struct ApiMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for ApiMiddlewareService<S>
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
        let api_key = req
            .headers()
            .get("X-API-KEY")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        
        if let Some(key) = api_key {
            req.extensions_mut().insert(key);  // Store API key as String
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

#[derive(Debug, Clone)]
pub struct ApiKey(pub String);

impl ApiKey {
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl FromRequest for ApiKey {
    type Error = crate::errors::AppError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let result = req
            .extensions()
            .get::<String>()
            .cloned()
            .map(ApiKey)
            .ok_or_else(|| crate::errors::AppError::Unauthorized("API key not provided".to_string()));
        
        ready(result)
    }
}
