# Networking Specification (v1)

This is the single source of truth for platform networking in v1.

It replaces repeated guidance across ingress, region, and tenant networking notes.

---

## Goals

Networking must:

* provide strict external routing for platform hosts
* provide region high availability without DNS churn
* provide tenant isolation by default
* keep implementation simple and operationally predictable

---

## Core Decisions

* Ingress implementation: Rust + Cloudflare Pingora
* Ingress package location: `packages/ingress`
* Edge TLS: Cloudflare (origin uses Full strict)
* Regional data plane: Kubernetes + Cilium (required in v1)
* Tenant isolation: Cilium network policies
* Global cross-region routing policy: out of scope for v1
* Customer custom domains: out of scope for v1

---

## Traffic Layers

1. Edge Layer
	* Cloudflare receives internet traffic and terminates public TLS.
	* Cloudflare forwards traffic to platform ingress origin over TLS.

2. Ingress Layer
	* Pingora service performs host-based routing.
	* Ingress reads routing metadata from control plane.

3. Regional Layer
	* Region traffic is sent to healthy cluster ingress endpoints.
	* Cilium provides in-region connectivity and load-balancing primitives.

4. Tenant Layer
	* Workloads communicate over tenant-scoped virtual networking.
	* Cilium policies enforce workspace isolation.

---

## External Host Routing (v1)

Supported hosts:

* `storage.<base-domain>` -> control-plane storage service
* `deployment.<base-domain>` -> control-plane API service
* `<region-slug>.<base-domain>` -> region ingress resolver
* `<deployment-id>.<region-slug>.<base-domain>` -> deployment route resolved inside the target region

Reserved host labels:

* `storage`
* `deployment`

`<deployment-id>` and `<region-slug>` values must not collide with reserved labels.

Unknown hosts return `404 Not Found`.

### Service Identity and Resolution

URL routing is a control-plane and ingress responsibility, not a Cilium responsibility.

Per request, ingress resolves:

1. target region
2. target deployment route
3. target service identity in that region

For `<deployment-id>.<region-slug>.<base-domain>` hosts, region is parsed from `<region-slug>` and deployment route key is parsed from `<deployment-id>`.

Service identity must be stored as a stable service reference (for example deployment + region + service name/port), not pod URLs.

Cilium then delivers traffic to healthy runtime endpoints for that selected service.

---

## Regional Ingress Model

Region DNS names must not point to a single cluster.

Each region is represented by:

* one logical region route
* multiple eligible cluster ingress endpoints

Per request to `<region-slug>.<base-domain>`:

1. resolve region slug
2. load cluster endpoints for region
3. filter to routable endpoints (healthy + ingress enabled)
4. select endpoint (weighted round-robin by default)
5. proxy request

If no endpoint is routable, return `503 Service Unavailable` with reason `NO_HEALTHY_CLUSTER_ENDPOINT`.

Region address stability uses Cilium-managed service VIP advertisement.

The Cilium advertisement mode (for example L2 or BGP) is an infrastructure implementation detail and does not change platform behavior.

---

## Cilium Responsibilities

Cilium is used for:

* cross-cluster connectivity inside a region (Cluster Mesh)
* service-level reachability across clusters
* regional ingress VIP advertisement
* network-policy enforcement for tenant isolation

Cilium is not the source of truth for product host routing.
Host routing policy remains in ingress/control-plane metadata.

---

## Tenant Isolation Model

Default security posture:

* deny cross-workspace traffic
* allow only explicitly permitted workspace/service traffic

Isolation identity is label-based and platform-managed.

Service discovery is stable at service identity level, not pod identity level.

---

## Control-Plane Metadata Contract

Ingress requires at minimum:

Region-level:

* region slug
* region status
* region routing mode (`active`, `draining`, `disabled`)

Cluster-level:

* cluster region id
* cluster ingress endpoint
* cluster ingress enabled flag
* cluster ingress weight
* cluster health/status

Recommended metadata refresh:

* poll every 5 seconds
* use last known good snapshot on refresh failure

### Metadata Population

Routing metadata is populated by the platform, not directly by Cilium.

Population flow:

1. Cluster agent watches Kubernetes ingress Service and cluster health signals.
2. Agent reports runtime networking state to control plane.
3. Control plane writes normalized routing fields to database.
4. Ingress service polls control-plane metadata and routes traffic.

Field population rules:

* `ingress_endpoint`: derived from cluster ingress Service VIP/external address.
* `ingress_enabled`: control-plane traffic gate (operator/policy/health driven).
* `ingress_weight`: control-plane balancing weight (default `100`, adjustable).

Cilium provides the networking primitives (VIP advertisement, service reachability, policy enforcement); the control plane remains the source of truth for routing metadata consumed by ingress.

---

## TLS and Edge Security

v1 TLS behavior:

* Cloudflare terminates public TLS
* Cloudflare -> origin uses Full strict
* origin presents Cloudflare origin certificate
* origin only accepts trusted edge traffic
* client IP is taken from Cloudflare forwarding headers

---

## Observability Requirements

Ingress must emit:

* request id
* host
* resolved region
* selected cluster endpoint
* upstream status and latency
* route failure reason

Required error reasons include at least:

* `NO_ROUTE`
* `NO_HEALTHY_CLUSTER_ENDPOINT`

---

## Non-Goals (v1)

* global host policy and cross-region runtime failover logic
* customer custom domains and cert onboarding
* advanced canary/shadow traffic controls

---

## Migration Note

Existing detailed notes may remain for historical context.
If any conflict exists, this document is authoritative for v1.
