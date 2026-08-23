use std::{
    net::{IpAddr, ToSocketAddrs},
    sync::Arc,
};

use async_trait::async_trait;
use bytes::Bytes;
use http::{HeaderName, header::HOST};
use pingora::{http::ResponseHeader, prelude::*, proxy::Session};

use crate::{
    config::Config,
    identity::{ClientIdentity, network_key, unspecified},
    limit::{LocalLimiter, Permit, RejectionKind},
    metrics::{Metrics, RequestMetrics},
    routing::{Decision, RouteMatch, Router},
};

pub struct Ingress {
    config: Arc<Config>,
    router: Router,
    identity: ClientIdentity,
    limiter: LocalLimiter,
    metrics: Metrics,
}

pub struct RequestContext {
    route: Option<RouteMatch>,
    client: IpAddr,
    authority: String,
    _permit: Option<Permit>,
    metrics: RequestMetrics,
}

impl Ingress {
    pub fn new(config: Config) -> prometheus::Result<Self> {
        let router = Router::new(config.authorities.clone());
        let identity = ClientIdentity::new(
            config.client_ip_header.clone(),
            config.trusted_proxies.clone(),
        );
        let limiter = LocalLimiter::new(config.limits.mode);
        let metrics = Metrics::register(prometheus::default_registry())?;
        Ok(Self {
            config: Arc::new(config),
            router,
            identity,
            limiter,
            metrics,
        })
    }
}

#[async_trait]
impl ProxyHttp for Ingress {
    type CTX = RequestContext;

    fn new_ctx(&self) -> Self::CTX {
        RequestContext {
            route: None,
            client: unspecified(),
            authority: String::new(),
            _permit: None,
            metrics: self.metrics.start_request(),
        }
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        let request = session.req_header();
        let authority = request
            .uri
            .authority()
            .map(|value| value.as_str())
            .or_else(|| {
                request
                    .headers
                    .get(HOST)
                    .and_then(|value| value.to_str().ok())
            })
            .unwrap_or_default()
            .to_owned();
        let peer = session
            .client_addr()
            .and_then(|address| address.as_inet())
            .map(|address| address.ip())
            .unwrap_or_else(unspecified);
        let client = network_key(self.identity.resolve(peer, &request.headers));

        ctx.authority = authority.clone();
        ctx.client = client;

        let route = match self
            .router
            .route(&authority, request.uri.path(), &request.method)
        {
            Decision::Proxy(route) => route,
            Decision::NotFound => {
                respond_json(session, 404, b"{\"error\":\"route not found\"}", None).await?;
                return Ok(true);
            }
            Decision::Misdirected => {
                respond_json(session, 421, b"{\"error\":\"unknown authority\"}", None).await?;
                return Ok(true);
            }
        };

        ctx.route = Some(route);
        match self
            .limiter
            .check(route.class, client, self.config.policy(route.class))
        {
            Ok(permit) => {
                if let Some(exceeded) = permit.exceeded() {
                    self.metrics.record_limit_exceeded(
                        route,
                        exceeded.kind,
                        self.config.limits.mode,
                    );
                }
                ctx._permit = Some(permit);
            }
            Err(rejection) => {
                self.metrics
                    .record_limit_exceeded(route, rejection.kind, self.config.limits.mode);
                self.metrics.record_rejection(route, rejection.kind);
                let (status, body) = match rejection.kind {
                    RejectionKind::RateLimit => {
                        (429, b"{\"error\":\"rate limit exceeded\"}".as_slice())
                    }
                    RejectionKind::Saturated => {
                        (503, b"{\"error\":\"ingress capacity exceeded\"}".as_slice())
                    }
                };
                respond_json(session, status, body, Some(rejection.retry_after_seconds)).await?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let route = ctx
            .route
            .expect("upstream_peer called before route selection");
        let upstream = self.config.upstreams.get(route.service);
        let address = upstream
            .to_socket_addrs()
            .map_err(|error| {
                Error::because(
                    ErrorType::ConnectError,
                    format!("could not resolve upstream {upstream:?}"),
                    error,
                )
            })?
            .next()
            .ok_or_else(|| {
                Error::explain(
                    ErrorType::ConnectError,
                    format!("upstream {upstream:?} did not resolve"),
                )
            })?;
        Ok(Box::new(HttpPeer::new(address, false, String::new())))
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut pingora::http::RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        let untrusted = upstream_request
            .headers
            .keys()
            .filter(|name| {
                let name = name.as_str();
                name.eq_ignore_ascii_case("forwarded")
                    || name.eq_ignore_ascii_case("x-real-ip")
                    || name.starts_with("x-forwarded-")
                    || name.starts_with("x-cplane-")
                    || self
                        .config
                        .client_ip_header
                        .as_deref()
                        .is_some_and(|header| name.eq_ignore_ascii_case(header))
            })
            .cloned()
            .collect::<Vec<HeaderName>>();
        for name in untrusted {
            upstream_request.remove_header(&name);
        }

        let client = ctx.client.to_string();
        upstream_request.insert_header("x-forwarded-for", &client)?;
        upstream_request.insert_header("x-real-ip", &client)?;
        upstream_request.insert_header("x-cplane-client-ip", &client)?;
        upstream_request.insert_header("x-forwarded-host", &ctx.authority)?;
        upstream_request.insert_header("x-forwarded-proto", &self.config.forwarded_proto)?;
        Ok(())
    }

    async fn logging(&self, session: &mut Session, error: Option<&Error>, ctx: &mut Self::CTX) {
        let status = session
            .response_written()
            .map_or(0, |response| response.status.as_u16());
        self.metrics.record(
            &ctx.metrics,
            ctx.route,
            status,
            session.body_bytes_read() as u64,
            session.body_bytes_sent() as u64,
        );
        if let Some(error) = error {
            self.metrics.record_proxy_error(ctx.route, error);
        }
        if let Some(route) = ctx.route {
            tracing::info!(
                route = route.id,
                class = %route.class,
                client = %ctx.client,
                status,
                error = error.map(|error| error.to_string()),
                "request complete"
            );
        }
    }
}

async fn respond_json(
    session: &mut Session,
    status: u16,
    body: &'static [u8],
    retry_after: Option<u64>,
) -> Result<()> {
    let mut response = ResponseHeader::build(status, Some(4))?;
    response.insert_header("content-type", "application/json")?;
    response.insert_header("cache-control", "no-store")?;
    response.insert_header("content-length", body.len().to_string())?;
    if let Some(seconds) = retry_after {
        response.insert_header("retry-after", seconds.to_string())?;
    }
    session
        .write_response_header(Box::new(response), false)
        .await?;
    session
        .write_response_body(Some(Bytes::from_static(body)), true)
        .await
}
