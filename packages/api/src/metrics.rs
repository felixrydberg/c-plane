use axum::{body::Body, extract::MatchedPath, http::Request, response::IntoResponse};
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Registry, TextEncoder,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use std::{sync::OnceLock, time::Instant};

pub struct Metrics {
    pub registry: Registry,
    pub http_requests: IntCounterVec,
    pub http_duration: HistogramVec,
    pub auth_requests: IntCounterVec,
    pub database_up: IntGaugeVec,
    pub platform_entities: IntGaugeVec,
    pub platform_metrics_up: prometheus::IntGauge,
}

static METRICS: OnceLock<Metrics> = OnceLock::new();

pub fn metrics() -> &'static Metrics {
    METRICS.get_or_init(|| {
        let registry = Registry::new();
        let http_requests = IntCounterVec::new(
            prometheus::Opts::new("cplane_api_http_requests_total", "HTTP requests"),
            &["method", "route", "status"],
        )
        .unwrap();
        let http_duration = HistogramVec::new(
            HistogramOpts::new(
                "cplane_api_http_request_duration_seconds",
                "HTTP request duration",
            ),
            &["method", "route"],
        )
        .unwrap();
        let auth_requests = IntCounterVec::new(
            prometheus::Opts::new("cplane_api_auth_requests_total", "Authentication requests"),
            &["mechanism", "outcome"],
        )
        .unwrap();
        let database_up = IntGaugeVec::new(
            prometheus::Opts::new("cplane_api_database_up", "Database query health"),
            &["database"],
        )
        .unwrap();
        let platform_entities = IntGaugeVec::new(
            prometheus::Opts::new("cplane_platform_entities", "Platform entity counts"),
            &["entity"],
        )
        .unwrap();
        let platform_metrics_up = prometheus::IntGauge::new(
            "cplane_api_platform_metrics_up",
            "Platform metrics collection health",
        )
        .unwrap();
        registry.register(Box::new(http_requests.clone())).unwrap();
        registry.register(Box::new(http_duration.clone())).unwrap();
        registry.register(Box::new(auth_requests.clone())).unwrap();
        registry.register(Box::new(database_up.clone())).unwrap();
        registry
            .register(Box::new(platform_entities.clone()))
            .unwrap();
        registry
            .register(Box::new(platform_metrics_up.clone()))
            .unwrap();
        Metrics {
            registry,
            http_requests,
            http_duration,
            auth_requests,
            database_up,
            platform_entities,
            platform_metrics_up,
        }
    })
}

pub async fn http_metrics(req: Request<Body>, next: axum::middleware::Next) -> impl IntoResponse {
    if req.uri().path() == "/metrics" {
        return next.run(req).await;
    }
    let method = req.method().as_str().to_owned();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str())
        .unwrap_or("unmatched")
        .to_owned();
    let started = Instant::now();
    let response = next.run(req).await;
    let status = response.status().as_u16().to_string();
    metrics()
        .http_requests
        .with_label_values(&[&method, &route, &status])
        .inc();
    metrics()
        .http_duration
        .with_label_values(&[&method, &route])
        .observe(started.elapsed().as_secs_f64());
    response
}

#[utoipa::path(get, path = "/metrics", tag = "metrics", responses((status = 200, description = "Prometheus metrics")))]
pub async fn endpoint() -> impl IntoResponse {
    collect_platform_metrics().await;
    let mut buffer = Vec::new();
    TextEncoder::new()
        .encode(&metrics().registry.gather(), &mut buffer)
        .unwrap();
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        buffer,
    )
}

async fn count(db: &DatabaseConnection, entity: &str, table: &str) -> Result<(), sea_orm::DbErr> {
    let filter = if entity == "active_api_keys" {
        " WHERE expires_at IS NULL OR expires_at = 0 OR created_at + make_interval(months => expires_at) > NOW()"
    } else {
        ""
    };
    let row = db
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT COUNT(*)::bigint AS count FROM {table}{filter}"),
        ))
        .await?;
    let value = row
        .ok_or_else(|| sea_orm::DbErr::Custom("missing count row".into()))?
        .try_get_by::<i64, _>("count")?;
    metrics()
        .platform_entities
        .with_label_values(&[entity])
        .set(value);
    Ok(())
}

async fn check_database(db: &DatabaseConnection, database: &str) -> bool {
    let healthy = db
        .execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT 1",
        ))
        .await
        .is_ok();
    metrics()
        .database_up
        .with_label_values(&[database])
        .set(i64::from(healthy));
    healthy
}

pub async fn collect_platform_metrics() {
    let state = crate::state::get_app_state();
    let identity_db = state.identity_db.connection();
    let mut healthy = check_database(identity_db, "identity").await;
    healthy &= check_database(&state.tenant_db, "tenant").await;

    // app_identity has read access across the shared database and bypasses RLS,
    // so global counts must use it rather than the tenant-scoped connection.
    for (entity, table) in [
        ("users", r#""user""#),
        ("organizations", r#""organization""#),
        ("memberships", r#""organization_member""#),
        ("active_api_keys", "api_keys"),
    ] {
        if count(identity_db, entity, table).await.is_err() {
            healthy = false;
            metrics()
                .database_up
                .with_label_values(&["identity"])
                .set(0);
        }
    }
    if count(identity_db, "projects", "project").await.is_err() {
        healthy = false;
        metrics()
            .database_up
            .with_label_values(&["identity"])
            .set(0);
    }
    metrics()
        .platform_metrics_up
        .set(if healthy { 1 } else { 0 });
}

pub fn auth(mechanism: &'static str, outcome: &'static str) {
    metrics()
        .auth_requests
        .with_label_values(&[mechanism, outcome])
        .inc();
}
