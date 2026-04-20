# Networking — Refined Decisions

## Public Ingress Runtime

- **Cilium Gateway API** is the ingress runtime inside each cluster.
- Handles host-based routing, HTTPRoutes, and TLS listeners per cluster.
- Control plane is the source of truth for routing policy; it reconciles Gateway and HTTPRoute resources.
- Cilium is not responsible for product host routing decisions.

## Public IP Assignment

- Ingress endpoints are registered when the cluster is created (or later updated) via control plane API.
- For on-prem clusters, operators can provide static IPs, VIPs, or DNS endpoints directly.
- For managed cloud provisioning, integrations (AWS/GCP/Hetzner) can populate endpoints automatically from provider responses.
- Endpoint inventory is control-plane-managed data, not auto-created from node discovery loops.

## Ingress Endpoint Model

- Each cluster tracks multiple ingress endpoints in a `cluster_ingress_endpoints` table.
- Each endpoint entry holds: address, port, enabled flag, health status, and last seen timestamp.
- The cluster agent periodically checks configured endpoints and reports health transitions.
- Health checks can update `health_status` and `last_seen_at` but do not create endpoint records.
- Ingress routes only to healthy endpoints.
- Node-level failover is handled by the platform, not by provider APIs or L2 VIP.
- Endpoint creation/update is handled by control plane API calls and provisioning integrations.

## Cluster-Level Failover

- A region has multiple clusters. If an entire cluster goes unhealthy, control plane routing policy removes it from active ingress.
- Ingress stops routing to that cluster. Traffic shifts to remaining healthy clusters in the region.
- Weighted routing across clusters is controlled by control plane policy.
- This gives regional HA without per-cluster VIP or provider-specific failover mechanics.

## Tenant Isolation

- Cilium Network Policy enforces isolation between tenant workloads by default.
- Default posture: deny cross-tenant traffic.
- Explicit policy edges (from service connection intent) are the only allowed paths.
- Control plane generates policy from the service connectivity graph, not from manual Cilium config.

## Internal Service Discovery

- Services are addressable by stable service identity (deployment + region + service name/port).
- Cilium Cluster Mesh provides cross-cluster service reachability within a region.
- Pod IPs are never used as stable references anywhere in the platform.

## What Cilium Owns

- Dataplane delivery to healthy endpoints.
- Cross-cluster connectivity within a region (Cluster Mesh).
- Network policy enforcement.
- LB IP assignment and Gateway exposure.

## What Cilium Does Not Own

- Product host routing decisions.
- Routing metadata (owned by control plane).
- Service identity (owned by control plane).
- Deployment placement (owned by scheduler).

## Per-Deployment Kubernetes Resources

Per-deployment Kubernetes resource definitions are part of the control-plane render contract and are documented in `docs/cluster/render.md`.

Networking consumes that rendered output and is responsible for dataplane delivery, not resource definition ownership.

## TLS

- TLS termination happens at the Cilium Gateway listener.
- Certificate lifecycle is managed per cluster. Custom domain TLS is out of scope for v1.
