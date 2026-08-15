# Containers

A container deployment is a user-defined workload running inside a platform-managed cluster. The user provides the image and config. The platform handles placement, scheduling, networking, and lifecycle.

## Deployment Config

**Required:**
- `image` — OCI image reference. Tags are resolved when a container version is created; deployments receive the immutable digest while the configured reference is retained.
- `region` — which region to deploy into.
- `port` — the port the container listens on.

**Optional:**
- `env` — environment variables. Each entry is either a plain value or a reference to a platform secret.
- `resources` — CPU and memory requests/limits. Defaults apply if not set (0.1 CPU / 128Mi).
- `replicas` — number of instances. Default: 1. Zero-downtime rolling updates require ≥ 2.
- `public` — whether to expose the deployment on a public hostname. Default: false.
- `external_registry_id` — optional organization-owned registry credentials for private images.
- `health_check` — readiness/liveness probe. HTTP path or TCP. Used by Kubernetes to determine pod health.

## Placement

- The control plane scheduler picks one or more clusters in the requested region.
- Cilium Cluster Mesh enables pods across clusters to share the same service identity, so multi-cluster deployments are a natural extension of the networking model.
- If a cluster becomes unhealthy, the control plane can reschedule affected deployments to other healthy clusters in the region.

## Kubernetes Resources

The cluster agent creates the following resources per deployment:

- `Deployment` — runs the workload pods. Runtime class is `kata-containers` (sandboxed by default).
- `Secret` — contains resolved env var values (plain + decrypted secret references). Owned by the deployment, deleted with it.
- `Service` — ClusterIP, stable in-cluster endpoint.
- `HTTPRoute` — created only if `public: true`. Routes the public hostname to the Service via the Cilium Gateway.

## Public Hostname

- Platform subdomain format: `<deployment-name>-<org-slug>.<region>.platform.dev`.
- Custom domains are out of scope for v1.
- TLS is terminated at the Cilium Gateway listener. Certificate lifecycle is managed per cluster.

## Lifecycle

- **Create**: scheduler picks cluster → agent reconciles → Deployment + Service + optional HTTPRoute created.
- **Update** (image, env, replicas, etc.): control plane updates desired state → agent triggers Kubernetes rolling update.
- **Delete**: control plane removes from desired state → agent deletes all associated resources.
- Manual changes to Kubernetes resources inside a cluster are overwritten on the next reconciliation cycle.

## Runtime

- All tenant workloads run under `kata-containers` runtime class by default.
- Kata provides VM-level isolation between tenants on shared nodes. Some Linux features are unavailable (specific syscalls, host mounts). Standard web workloads are unaffected.
