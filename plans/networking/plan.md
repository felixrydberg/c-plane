# Networking + Ingress Implementation Plan (v1)

This plan delivers the ingress package and its control-plane metadata supply path end-to-end.

It follows the authoritative v1 specs:

- specification/infrastructure/ingres.md
- specification/infrastructure/networking.md

## Scope

In scope:

- new standalone Rust ingress package at packages/ingress
- strict host-based routing for platform hosts
- c-plane metadata endpoint for ingress polling
- system-to-system auth for ingress metadata polling
- local/dev deployment wiring and verification

Out of scope (v1):

- custom domains
- cross-region failover policy
- canary/shadow traffic controls
- tenant certificate lifecycle

## Decisions

- Metadata source: c-plane HTTP endpoint (ingress polls every 5 seconds).
- Auth baseline: scoped API key for ingress -> c-plane metadata polling.
- Network restrictions: optional hardening, not primary identity (important for self-hosting).
- Upstream protocol: endpoint scheme configurable (HTTP or HTTPS).
- Execution mode: parallel tracks (ingress runtime + control-plane supply), then integration.
- Service routing model: ingress resolves region + deployment route + service identity; Cilium handles endpoint delivery.

## Phase 0: Contract Lock (Blocking)

Goal: finalize the contract both tracks build against.

Tasks:

1. Lock metadata response contract.
2. Lock route failure/error semantics.
3. Lock auth requirement and scope naming.
4. Lock service identity contract used by ingress routing.

Required metadata fields:

- Region: slug, status, routing_mode
- Cluster: region_id, ingress_endpoint, ingress_enabled, ingress_weight, health_status

Required service identity contract:

- deployment_id
- region_id or region_slug
- service_name
- service_port
- host/path match key used by ingress lookup

Host parsing contract additions:

- for `<deployment-id>.<region-slug>.<base-domain>` hosts:
	- `deployment_id` comes from first label
	- `region_slug` comes from second label

Required route outcomes:

- Unknown host -> 404 Not Found
- No routable cluster endpoint -> 503 Service Unavailable with reason NO_HEALTHY_CLUSTER_ENDPOINT

Deliverables:

- contract note section in this plan (completed by this document)
- TODO comments in implementation files where contract is consumed/exposed

## Phase 1A: Ingress Runtime (packages/ingress)

Goal: implement Pingora ingress service behavior.

Tasks:

1. Create package scaffold:
	- packages/ingress/Cargo.toml
	- packages/ingress/src/main.rs
	- packages/ingress/src/config/mod.rs
	- packages/ingress/src/errors/mod.rs
	- packages/ingress/src/handlers/health.rs
	- packages/ingress/src/routes/mod.rs
	- packages/ingress/src/services/metadata_client.rs
	- packages/ingress/src/services/region_router.rs
2. Implement host classifier for:
	- storage.<base-domain>
	- deployment.<base-domain>
	- <deployment-id>.<region-slug>.<base-domain>
3. Implement routing behavior:
	- storage/deployment host routing to control-plane services
	- region host routing through metadata snapshot and endpoint selection
	- service identity lookup (deployment + region -> service target)
4. Implement routable endpoint filter:
	- ingress_enabled == true
	- cluster health is routable
	- region routing_mode allows routing
5. Implement weighted endpoint selection.
6. Implement metadata poller:
	- poll every 5s
	- atomic snapshot swap on success
	- retain last-known-good snapshot on refresh failure
7. Implement observability and health endpoints.

Acceptance criteria:

- host routes behave exactly per spec
- region routing returns 503 + NO_HEALTHY_CLUSTER_ENDPOINT when no endpoint is routable
- metadata refresh failures do not immediately drop routing if LKG snapshot exists

## Phase 1B: Control-Plane Metadata Supply (c-plane + admin API gaps)

Goal: provide stable, secure routing metadata to ingress.

Tasks:

1. Add c-plane metadata endpoint for ingress polling.
	- include only fields ingress requires
	- response optimized for frequent polling
2. Add auth check for dedicated ingress system scope (API key).
3. Close control-plane updateability gaps:
	- add routing_mode to region PATCH payload handling
	- add ingress_endpoint to cluster PATCH payload handling
	- add ingress_enabled to cluster PATCH payload handling
	- add ingress_weight to cluster PATCH payload handling
4. Ensure response error shape is predictable for ingress client.

Acceptance criteria:

- ingress-scoped key can fetch metadata
- missing/invalid key is denied
- metadata reflects admin/API updates for region/cluster routing fields

## Phase 2: Integration Wiring

Goal: run ingress and c-plane together in local/dev and verify behavior.

Tasks:

1. Add packages/ingress to Rust workspace members.
2. Add ingress service to docker-compose.dev.yml.
3. Ensure Dockerfile.rust supports PACKAGE=ingress cleanly.
4. Add ingress env vars to .env.example:
	- INGRESS_SERVER_HOST
	- INGRESS_SERVER_PORT
	- INGRESS_BASE_DOMAIN
	- INGRESS_METADATA_URL
	- INGRESS_METADATA_API_KEY
	- INGRESS_METADATA_POLL_INTERVAL_SECONDS (default 5)
5. Wire service discovery targets for storage/deployment control-plane paths.

Acceptance criteria:

- local compose stack starts with ingress
- ingress can poll metadata from c-plane
- host routing works in dev for all supported host classes

## Phase 3: Verification and Readiness

Goal: validate correctness and operational confidence.

Tests to add/run:

1. Host parsing and reserved label behavior.
2. Unknown host returns 404.
3. Endpoint eligibility filter behavior.
4. Weighted endpoint selection determinism.
5. Poller LKG fallback behavior.
6. Metadata endpoint auth checks.
7. End-to-end region routing success path.
8. End-to-end no-routable-endpoint failure path (503 + reason).

Observability checks:

- request id emitted
- host emitted
- resolved region emitted
- selected cluster endpoint emitted
- upstream status/latency emitted
- route failure reason emitted

## Change List (Planned)

- Cargo.toml (workspace member add)
- docker-compose.dev.yml (ingress service add)
- Dockerfile.rust (verify ingress package compatibility)
- .env.example (ingress vars)
- packages/ingress/** (new package implementation)
- packages/c-plane/src/routes/mod.rs (metadata route)
- packages/c-plane/src/handlers/** (metadata handler)
- packages/c-plane/src/middleware/auth.rs (ingress scope auth)
- packages/ui-studio/server/api/infrastructure/regions/[region_id].patch.ts
- packages/ui-studio/server/api/infrastructure/clusters/[cluster_id].patch.ts

## Risks and Mitigations

1. Risk: metadata contract drift between c-plane and ingress.
	Mitigation: keep shared typed contract module and test fixture snapshots.
2. Risk: stale metadata during control-plane outages.
	Mitigation: LKG cache with clear staleness logging and alerting.
3. Risk: self-hosted deployments lack stable source-IP controls.
	Mitigation: treat API key as mandatory identity; make network restrictions optional hardening.

## Execution Order

1. Phase 0 contract lock
2. Phase 1A and Phase 1B in parallel
3. Phase 2 integration wiring
4. Phase 3 verification/readiness

## Definition of Done

Done means:

1. packages/ingress exists and runs in local compose.
2. c-plane exposes authenticated metadata endpoint consumed by ingress every 5 seconds.
3. routing behavior matches v1 spec for known/unknown hosts and no-endpoint conditions.
4. required tests pass and logs include required routing/decision fields.
