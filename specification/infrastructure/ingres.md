# Ingress Specification (v1, Simple)

This document defines the ingress behavior for v1.
The objective is implementation simplicity.

Global traffic behavior is intentionally out of scope in this version.

---

## Goals

Ingress must:

* provide strict host-based routing for platform hosts
* support region-pinned traffic only
* keep TLS and operations simple
* avoid custom domains in v1
* standardize implementation on Rust and Cloudflare Pingora

---

## In Scope (v1)

* platform-provided domains
* host-based routing
* region-host routing (`<region-slug>.<base-domain>`)
* Cloudflare TLS offload
* Pingora-based ingress runtime

## Implementation Stack

Ingress is implemented as a dedicated Rust service using Cloudflare Pingora.

Required choices for v1:

* language: Rust
* proxy/runtime: Cloudflare Pingora
* deployment unit: standalone ingress package

## Package Structure

Ingress must be developed as a new package under the monorepo:

* `packages/ingress`

Responsibilities of this package:

* host parsing and routing decisions
* control-plane route metadata polling/cache
* proxying to control-plane services and regional endpoints
* ingress metrics and health endpoints

## Out of Scope (v1)

* global host routing and cross-region runtime failover policy
* customer custom domains
* tenant certificate management
* advanced traffic splitting/canary
* migrating ingress logic into existing `c-plane` package

---

## Host Model

Ingress must support:

* `storage.<base-domain>`
* `deployment.<base-domain>`
* `<region-slug>.<base-domain>`

Reserved labels:

* `storage`
* `deployment`

Unknown hosts must return `404 Not Found`.

---

## Routing Rules

Required host routing:

* `storage.<base-domain>` -> control-plane storage service
* `deployment.<base-domain>` -> control-plane deployment/API service
* `<region-slug>.<base-domain>` -> region route resolver

For region routes, ingress resolves region endpoints from control-plane metadata and proxies to healthy cluster ingress endpoints in that region.

Regional ingress addressing uses Cilium-managed VIP/service exposure.

If no healthy endpoint exists, ingress must return `503 Service Unavailable` with reason `NO_HEALTHY_CLUSTER_ENDPOINT`.

---

## TLS Model

v1 uses Cloudflare-managed edge TLS:

* Cloudflare terminates public TLS
* Cloudflare to origin uses `Full (strict)`
* origin presents Cloudflare Origin Certificate
* origin accepts traffic only from trusted edge ranges

This avoids region-by-region public certificate lifecycle in v1.

---

## Control-Plane Routing Metadata

Ingress consumes routing metadata from control plane.

Minimum required fields:

* region slug
* region status/routing mode
* cluster ingress endpoint(s)
* cluster ingress enabled flag
* cluster ingress weight

Recommended refresh behavior:

* poll every 5 seconds
* retain last known good snapshot on refresh failure

---

## Observability Requirements

Ingress must emit:

* request id
* host
* resolved region
* selected cluster endpoint
* upstream status and latency
* route failure reason

---

## Related Network Specifications

Detailed routing behavior for regional traffic is defined in:

* [specification/infrastructure/network/region.md](specification/infrastructure/network/region.md)

Tenant isolation and service-to-service networking context is defined in:

* [specification/infrastructure/network/organization-network.md](specification/infrastructure/network/organization-network.md)

Historical/extended ingress notes are in:

* [specification/infrastructure/network/ingress.md](specification/infrastructure/network/ingress.md)

When statements conflict, this v1 document is authoritative for current implementation scope.
