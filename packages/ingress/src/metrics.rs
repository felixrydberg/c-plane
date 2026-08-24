use std::time::Instant;

use pingora::{Error, ErrorSource};
use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Opts, Registry};

use crate::{
    limit::{LimitMode, RejectionKind},
    routing::RouteMatch,
};

#[derive(Clone)]
pub struct Metrics {
    requests: IntCounterVec,
    duration: HistogramVec,
    body_bytes: IntCounterVec,
    in_flight: IntGauge,
    rejections: IntCounterVec,
    limit_exceeded: IntCounterVec,
    proxy_errors: IntCounterVec,
}

pub struct RequestMetrics {
    started_at: Instant,
    in_flight: IntGauge,
}

impl Metrics {
    pub fn register(registry: &Registry) -> prometheus::Result<Self> {
        let requests = IntCounterVec::new(
            Opts::new(
                "cplane_ingress_requests_total",
                "Completed ingress requests.",
            ),
            &["route", "class", "status"],
        )?;
        let duration = HistogramVec::new(
            HistogramOpts::new(
                "cplane_ingress_request_duration_seconds",
                "Ingress request duration in seconds.",
            )
            .buckets(vec![
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 300.0,
            ]),
            &["route", "class"],
        )?;
        let body_bytes = IntCounterVec::new(
            Opts::new(
                "cplane_ingress_body_bytes_total",
                "HTTP body bytes transferred through ingress.",
            ),
            &["route", "class", "direction"],
        )?;
        let in_flight = IntGauge::new(
            "cplane_ingress_requests_in_flight",
            "Ingress requests currently in flight.",
        )?;
        let rejections = IntCounterVec::new(
            Opts::new(
                "cplane_ingress_rejections_total",
                "Ingress requests rejected by local limits.",
            ),
            &["class", "reason"],
        )?;
        let limit_exceeded = IntCounterVec::new(
            Opts::new(
                "cplane_ingress_limit_exceeded_total",
                "Local ingress limit exceedances, including observe mode.",
            ),
            &["class", "reason", "mode"],
        )?;
        let proxy_errors = IntCounterVec::new(
            Opts::new(
                "cplane_ingress_proxy_errors_total",
                "Fatal ingress proxy errors by stable Pingora source and kind.",
            ),
            &["route", "class", "source", "kind"],
        )?;

        registry.register(Box::new(requests.clone()))?;
        registry.register(Box::new(duration.clone()))?;
        registry.register(Box::new(body_bytes.clone()))?;
        registry.register(Box::new(in_flight.clone()))?;
        registry.register(Box::new(rejections.clone()))?;
        registry.register(Box::new(limit_exceeded.clone()))?;
        registry.register(Box::new(proxy_errors.clone()))?;

        Ok(Self {
            requests,
            duration,
            body_bytes,
            in_flight,
            rejections,
            limit_exceeded,
            proxy_errors,
        })
    }

    pub fn start_request(&self) -> RequestMetrics {
        self.in_flight.inc();
        RequestMetrics {
            started_at: Instant::now(),
            in_flight: self.in_flight.clone(),
        }
    }

    pub fn record(
        &self,
        request: &RequestMetrics,
        route: Option<RouteMatch>,
        status: u16,
        request_bytes: u64,
        response_bytes: u64,
    ) {
        let (route, class) = route
            .map(|route| (route.id, route.class.as_str()))
            .unwrap_or(("unmatched", "unmatched"));
        let status = status.to_string();

        self.requests
            .with_label_values(&[route, class, &status])
            .inc();
        self.duration
            .with_label_values(&[route, class])
            .observe(request.started_at.elapsed().as_secs_f64());
        self.body_bytes
            .with_label_values(&[route, class, "request"])
            .inc_by(request_bytes);
        self.body_bytes
            .with_label_values(&[route, class, "response"])
            .inc_by(response_bytes);
    }

    pub fn record_rejection(&self, route: RouteMatch, reason: RejectionKind) {
        self.rejections
            .with_label_values(&[route.class.as_str(), reason.as_str()])
            .inc();
    }

    pub fn record_limit_exceeded(&self, route: RouteMatch, reason: RejectionKind, mode: LimitMode) {
        self.limit_exceeded
            .with_label_values(&[route.class.as_str(), reason.as_str(), mode.as_str()])
            .inc();
    }

    pub fn record_proxy_error(&self, route: Option<RouteMatch>, error: &Error) {
        let (route, class) = route
            .map(|route| (route.id, route.class.as_str()))
            .unwrap_or(("unmatched", "unmatched"));
        let source = match error.esource() {
            ErrorSource::Upstream => "upstream",
            ErrorSource::Downstream => "downstream",
            ErrorSource::Internal => "internal",
            ErrorSource::Unset => "unset",
        };
        self.proxy_errors
            .with_label_values(&[route, class, source, error.root_etype().as_str()])
            .inc();
    }
}

impl Drop for RequestMetrics {
    fn drop(&mut self) {
        self.in_flight.dec();
    }
}
