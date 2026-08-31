package telemetry

import "github.com/prometheus/client_golang/prometheus/promauto"
import "github.com/prometheus/client_golang/prometheus"

var (
	ResolverRequests = promauto.NewCounterVec(prometheus.CounterOpts{
		Namespace: "cplane_registry", Name: "resolver_requests_total",
		Help: "Managed Registry metadata resolver requests.",
	}, []string{"result"})
	ResolverLatency = promauto.NewHistogram(prometheus.HistogramOpts{
		Namespace: "cplane_registry", Name: "resolver_duration_seconds",
		Help: "Managed Registry metadata resolver latency.",
	})
	AuthenticationFailures = promauto.NewCounter(prometheus.CounterOpts{
		Namespace: "cplane_registry", Name: "authentication_failures_total",
		Help: "Rejected Registry bearer tokens and scopes.",
	})
	WriteRejections = promauto.NewCounter(prometheus.CounterOpts{
		Namespace: "cplane_registry", Name: "write_rejections_total",
		Help: "Registry mutations rejected by organization-scoped GC gating.",
	})
	CacheEvents = promauto.NewCounterVec(prometheus.CounterOpts{
		Namespace: "cplane_registry", Name: "driver_cache_events_total",
		Help: "Organization Storage driver cache events.",
	}, []string{"event"})
	Redirects = promauto.NewCounterVec(prometheus.CounterOpts{
		Namespace: "cplane_registry", Name: "redirects_total",
		Help: "Storage redirect attempts.",
	}, []string{"result"})
	OperationLatency = promauto.NewHistogramVec(prometheus.HistogramOpts{
		Namespace: "cplane_registry", Name: "storage_operation_duration_seconds",
		Help: "Delegated storage operation latency.",
	}, []string{"operation", "result"})
	Bytes = promauto.NewCounterVec(prometheus.CounterOpts{
		Namespace: "cplane_registry", Name: "storage_bytes_total",
		Help: "Bytes passed through non-streaming delegated storage operations.",
	}, []string{"direction"})
)
