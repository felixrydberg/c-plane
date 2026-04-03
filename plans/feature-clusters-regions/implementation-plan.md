# Regions and Clusters Feature Implementation Plan

## Scope

Implement region and cluster management for the platform with strict separation of concerns:

- `ui-studio` only exposes admin API proxy routes.
- Rust `c-plane` is the source of truth for infrastructure persistence and business logic.
- Organizations remain UI-side concerns and are not migrated into Rust for this feature.
- Auth direction is `Better Auth users + API keys`; Kratos is being removed.

## Confirmed Architecture Decisions

1. `ui-studio` forwards requests to Rust instead of writing infra data directly.
2. Rust endpoints for this feature live under `/admin/infrastructure/...`.
3. `ui-studio -> Rust` auth uses an internal service token.
4. `ui-studio` forwards actor context headers for audit/authorization checks.
5. Rust implementation aligns with future `IdentityRepository`, `TenantRepository`, and `InfrastructureRepository` split.

## Functional Scope

### Regions
- Create
- List
- Update
- Delete

### Clusters
- Create
- List
- Update
- Delete

### Additional Data Scope
- Region capability mapping (table-level support now; policy checks can be expanded later)
- Cluster capacity and health fields

## Phase Plan

### Phase 1: API Contract and Auth Boundary

Deliverables:
- Request/response schema for regions/clusters CRUD.
- Error envelope contract for Rust responses.
- Forwarding contract from studio (headers, query, body passthrough).

Key decisions:
- Actor headers: `x-actor-id`, `x-actor-email`, `x-actor-role`.
- Internal auth header: `Authorization: Bearer <CPLANE_INTERNAL_TOKEN>`.

### Phase 2: Data Model in ui-shared

Deliverables:
- Add schema folders:
  - `schema/regions/schema.ts`
  - `schema/clusters/schema.ts`
  - `schema/infrastructure-capabilities/schema.ts`
- Export all new schemas in `schema/index.ts`.
- Generate migration in `packages/ui-shared/drizzle`.

Minimum model fields:
- Region: id, slug, display_name, status, timestamps.
- Cluster: id, region_id, slug, name, kube_api_endpoint, status, capacity_allocatable, capacity_used, health_status, last_heartbeat_at, timestamps.
- Capability mapping: region-to-scope assignment table(s).

### Phase 3: Rust c-plane Infrastructure API

Deliverables:
- Route group under `/admin/infrastructure`:
  - `/regions`
  - `/regions/:region_id`
  - `/clusters`
  - `/clusters/:cluster_id`
- Handlers + services for CRUD.
- Infrastructure-focused repository abstraction (or initial module boundary) compatible with future repository split.

Auth changes for this route group:
- Add service-token middleware.
- Validate forwarded actor role is admin.
- Return consistent 401/403 on invalid token/role.

### Phase 4: ui-studio Proxy Endpoints

Deliverables:
- Add proxy routes in `packages/ui-studio/server/api/admin/`:
  - `regions/*`
  - `clusters/*`
- Add proxy utility for Rust forwarding.
- Add admin session check helper in `server/utils/authorization.ts`.

Behavior:
- `ui-studio` validates Better Auth session + admin role first.
- Proxy forwards method, path, query, and body unchanged.
- Proxy maps Rust error envelope to Nuxt `createError` consistently.

### Phase 5: Validation and Rollout

Deliverables:
- Rust tests for CRUD and auth failures.
- Studio server tests for proxy forwarding behavior.
- Manual end-to-end verification from studio APIs to Rust persistence.

Acceptance checks:
- Regions CRUD succeeds through `ui-studio -> Rust`.
- Clusters CRUD succeeds through `ui-studio -> Rust`.
- No direct infra writes in `ui-studio` DB layer.
- Invalid service token fails.
- Non-admin user fails before forwarding.

## Files Expected To Change

### ui-shared
- `packages/ui-shared/schema/index.ts`
- `packages/ui-shared/schema/regions/schema.ts`
- `packages/ui-shared/schema/clusters/schema.ts`
- `packages/ui-shared/schema/infrastructure-capabilities/schema.ts`
- `packages/ui-shared/drizzle/*` (generated migration)

### c-plane (Rust)
- `packages/c-plane/src/routes/mod.rs`
- `packages/c-plane/src/middleware/auth.rs` (or split middleware module)
- `packages/c-plane/src/handlers/*` (new infrastructure handlers)
- `packages/c-plane/src/services/*` (new infrastructure services)
- `packages/c-plane/src/models/entities/*` (new infrastructure entities)
- `packages/c-plane/src/errors/*` (infra/auth variants as needed)

### ui-studio
- `packages/ui-studio/server/utils/authorization.ts`
- `packages/ui-studio/server/utils/*` (new proxy helper)
- `packages/ui-studio/server/api/admin/regions/*`
- `packages/ui-studio/server/api/admin/clusters/*`

## Out of Scope

- Full scheduler/reconciliation runtime implementation.
- Frontend page completion beyond API requirements.
- Full global auth migration in Rust outside this feature path.

## Risks and Mitigations

1. Auth mismatch during Kratos removal:
- Mitigation: isolate service-token middleware for this route group immediately.

2. Tight coupling between studio and Rust error formats:
- Mitigation: define error envelope contract in Phase 1 and lock it.

3. Tenant/repository refactor drift:
- Mitigation: keep clear module boundaries so infrastructure code can be moved under `InfrastructureRepository` without API changes.
