## Plan: Cloud Billing Foundation for Future Plans

Establish a cloud-only, capability-first billing foundation that persists plan entitlements on the organization and supports future usage-based meters without committing to meter math yet. The first milestone is a durable data model + Polar sync + enforcement hooks so plan changes become data updates rather than code rewrites.

**Steps**
1. Phase 1 - Data Contract and Schema Foundation
2. Define canonical billing capability shape for organizations in shared types: onboarding state, billing mode, limits, features, and placeholder meter catalog. Include explicit "cloud_only" assumptions and default values for new orgs. *blocks steps 3-10*
3. Add organization schema fields and enums in shared Drizzle schema: onboarding/setup status, capability payload, billing status summary, and external billing identifiers that can store Polar-native IDs as strings. Keep existing fields for compatibility during migration. *depends on 2*
4. Add new subscription state table(s) for organization-level billing linkage and webhook audit/idempotency tracking. Include period boundaries and status snapshots; do not add usage math tables yet. *depends on 3*
5. Add migration(s) to backfill existing organizations with cloud defaults and safe transitional values; ensure no destructive changes. *depends on 3-4*

6. Phase 2 - Server APIs and Polar Integration Skeleton
7. Implement billing domain utilities in server layer: organization capability resolver, cloud-mode guard, and plan assignment mapper (Plan -> capabilities), with mapper using static config for now. *depends on 5*
8. Implement/repair Polar client utility and environment validation startup checks; fail safely when cloud billing is enabled but required keys are missing. *parallel with 7*
9. Add billing endpoints for v1 scaffolding: get organization billing state, assign/update plan capability set (internal/admin flow), and customer portal redirect route used by settings navigation. *depends on 7-8*
10. Add Polar webhook endpoint skeleton with signature validation, idempotent event storage, and subscription-status projection updates onto organization billing state. *depends on 4,8*

11. Phase 3 - Runtime Enforcement Hooks (No Meter Math Yet)
12. Integrate organization capability checks into authorization helpers for write operations (resource creation paths), returning consistent "limit/capability not allowed" errors. Apply prospectively only; never delete existing resources. *depends on 7*
13. Add onboarding/billing gating rule so organizations not operational for cloud billing remain blocked from normal workflows while still allowing required setup actions. *depends on 7,9*
14. Add placeholder usage event interface and ingestion hook contracts (CPU/RAM/storage/deployment-type dimensions) without invoice computation, so metering can be added later without schema redesign. *depends on 4,7*

15. Phase 4 - UI/Type Wiring and Operational Safety
16. Extend organization API responses and shared frontend types so app/store receives effective capabilities and billing summary from backend source of truth. *depends on 9,13*
17. Wire billing settings navigation to real portal/state endpoints and show deterministic entitlement state (capabilities-driven, not plan-name checks in components). *depends on 9,16*
18. Add structured logs and minimal observability around webhook processing and capability resolution decisions for debugging and support. *parallel with 16-17*

**Relevant files**
- c:/Users/balle/Documents/GitHub/c-plane/packages/migrations/schema/organization/schema.ts - add organization billing/capability columns, enums, and relations
- c:/Users/balle/Documents/GitHub/c-plane/packages/migrations/schema/index.ts - export new billing/subscription schema modules
- c:/Users/balle/Documents/GitHub/c-plane/packages/migrations/drizzle/0000_damp_ultimo.sql - reference baseline migration patterns; add follow-up migration files
- c:/Users/balle/Documents/GitHub/c-plane/packages/ui/shared/types/organization.ts - extend organization type for onboarding/capabilities/billing summary
- c:/Users/balle/Documents/GitHub/c-plane/packages/ui/server/api/organization/index.post.ts - initialize cloud defaults on org creation
- c:/Users/balle/Documents/GitHub/c-plane/packages/ui/server/api/organization/active.get.ts - return effective billing/capability state in active org payload
- c:/Users/balle/Documents/GitHub/c-plane/packages/ui/server/utils/authorization.ts - add capability enforcement helper hooks
- c:/Users/balle/Documents/GitHub/c-plane/packages/ui/app/layouts/auth.vue - currently calls portal route; align with implemented endpoint contract
- c:/Users/balle/Documents/GitHub/c-plane/packages/ui/server/api/organization/[organization_id]/index.delete.ts - use validated Polar identifier fields and hardened error handling
- c:/Users/balle/Documents/GitHub/c-plane/packages/ui/nuxt.config.ts - confirm Polar module/runtime config for cloud billing mode
- c:/Users/balle/Documents/GitHub/c-plane/packages/ui/server/utils (new billing utilities area) - capability resolver, plan mapper, cloud guard, Polar client initialization
- c:/Users/balle/Documents/GitHub/c-plane/packages/ui/server/api (new billing and Polar webhook routes) - billing state, portal, webhook ingestion

**Verification**
1. Run Drizzle migration generation/apply and confirm organization/subscription data exists for pre-existing rows with expected defaults.
2. Validate org creation path populates onboarding/billing/capability defaults and returns them in active organization payload.
3. Exercise billing portal route from settings navigation and confirm deterministic error when billing mode is disabled/misconfigured.
4. Send signed test webhook payloads (valid, duplicate, invalid signature) and verify idempotency storage and projected subscription state updates.
5. Execute write-operation API tests for a capped organization and verify prospective enforcement behavior (existing resources preserved, new creations blocked when applicable).
6. Confirm frontend behavior uses capability fields from API rather than hardcoded plan checks.

**Decisions**
- Included scope: cloud-only billing foundation, capability persistence, Polar sync skeleton, enforcement hooks, UI wiring to server truth.
- Excluded scope: final pricing catalog, meter formulas, invoice math, full usage aggregation pipeline, Rust-side enforcement in c-plane.
- Billing authority: organization effective capabilities in database are the runtime source of truth; plan is mapping input only.
- Safety rule: downgrades are prospective-only and non-destructive.

**Further Considerations**
1. Polar identifier migration: migrate organization.polar_customer_id from UUID semantics to string-compatible external ID strategy to avoid API mismatch risk.
2. Environment mode contract: introduce explicit runtime flag for cloud billing mode instead of implicit behavior.
3. Event ownership: decide whether metering ingestion remains in Nuxt server or later moves into Rust service boundaries once deployment runtime model lands.
