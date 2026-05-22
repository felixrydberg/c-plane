## Plan: RLS Rollout for ui + control-plane-ui

Enable PostgreSQL RLS end-to-end for Nuxt server APIs in ui and control-plane-ui using Drizzle-defined roles/policies, with request-scoped tenant context and phased migration from application-side organization filters to database-enforced isolation.

**Goals**
- Keep schema authority in ui-shared using Drizzle declarations and generated migrations.
- Ensure every tenant query runs under app_tenant role with app.allowed_organizations set per request/transaction.
- Preserve behavior during migration with no cross-tenant regression.
- Keep identity/auth queries on app_identity role and never bypass tenant context for data tables.

**Outcomes**
- ui and control-plane-ui server routes are RLS-safe.
- Manual organization_id filtering is minimized to identity-resolution boundaries.
- Access control for data isolation is enforced by database policies.

**Phase 0 - Discovery and Classification**
1. Inventory all server endpoints in ui and control-plane-ui and map each to tables touched.
2. Classify each table as identity-scope (auth/member lookup) vs tenant-scope (organization data).
3. Mark routes that currently depend on application-level filters and where RLS can replace them directly.
4. Document exceptions (global catalog tables like regions/clusters) that should not use tenant RLS.

**Phase 1 - Shared RLS Contract in ui-shared**
5. Define shared roles in Drizzle schema (app_identity, app_tenant, app_admin) and keep in ui-shared exports.
6. Add/adjust tenant policies with Drizzle pgPolicy + enableRLS only on tenant tables used by ui/control-plane-ui.
7. For append-only tables (event), keep insert-only tenant policy.
8. For invitation/event access patterns, decide explicit per-operation policies based on real route usage from Phase 0.
9. Generate Drizzle migration from schema changes; avoid handwritten policy SQL.

**Phase 2 - Request-Scoped DB Context in Nuxt Servers**
10. Add DB context helpers in ui and control-plane-ui server utils:
- getIdentityDb(): identity-role connection for membership/key/session resolution.
- withTenantDb(event, fn): opens transaction, sets SET LOCAL app.allowed_organizations, runs fn, commits/rolls back.
11. Ensure SET LOCAL is always executed inside a transaction boundary (pgBouncer-safe behavior).
12. Add typed organization context object (actor_id, allowed_organizations, auth_type, scopes).
13. Rework authorization helper to resolve membership first (identity scope), then execute tenant work via withTenantDb.

**Phase 3 - Route Migration (ui)**
14. Migrate organization APIs first (active org, membership checks, invitations, api keys).
15. Replace direct db usage in handlers with tenant context helper for tenant tables.
16. Keep explicit org filters only where required for business semantics, not isolation correctness.
17. Validate API key flows and scope checks against tenant-scoped writes.

**Phase 4 - Route Migration (control-plane-ui)**
18. Apply same tenant context abstraction in control-plane-ui server.
19. Migrate organization and studio-specific tenant routes.
20. Preserve admin-only behavior by combining identity role checks with tenant context where data is tenant-scoped.

**Phase 5 - Policy Hardening and Cleanup**
21. Remove redundant app-layer isolation filters after equivalent RLS coverage is validated.
22. Ensure no tenant route can execute against raw connection without context initialization.
23. Add lightweight guardrails (helper usage patterns, lint comments, or tests) to prevent bypass.

**Phase 6 - Verification and Rollout**
24. Add integration tests for both apps:
- cross-org read/write denial
- allowed-org reads/writes
- append-only event behavior
- invitation behavior according to finalized policy decisions
25. Run migration + smoke tests in local/dev environments for ui and control-plane-ui.
26. Stage rollout:
- policy creation first
- route migration second
- app-filter cleanup last
27. Add observability logs for actor_id, allowed_org_count, and route policy denials (without leaking secrets).

**Open Decisions To Finalize Early**
1. organization_invitation policy shape:
- insert-only
- select+insert
- full CRUD
based on actual route behavior in ui and control-plane-ui.
2. Whether api_keys/api_key_scopes should be tenant role writable from Nuxt or remain identity-scope operations.
3. Whether event reads should be allowed for tenants or remain write-only ingest in current phase.

**Implementation Checklist**
- [ ] Complete endpoint/table inventory for ui.
- [ ] Complete endpoint/table inventory for control-plane-ui.
- [ ] Finalize per-table policy matrix.
- [ ] Generate and review Drizzle migration SQL.
- [ ] Migrate ui server routes to tenant helper.
- [ ] Migrate control-plane-ui server routes to tenant helper.
- [ ] Add integration tests and run full pass.

**Primary Files To Touch**
- packages/ui-shared/schema/rls.ts
- packages/ui-shared/schema/organization/schema.ts
- packages/ui-shared/schema/events/schema.ts
- packages/ui-shared/schema/api-keys/schema.ts
- packages/ui-shared/schema/index.ts
- packages/ui/server/utils/authorization.ts
- packages/ui/server/utils/db.ts (or equivalent helper module)
- packages/ui/server/api/**
- packages/control-plane-ui/server/utils/authorization.ts
- packages/control-plane-ui/server/utils/db.ts (or equivalent helper module)
- packages/control-plane-ui/server/api/**

**Success Criteria**
- Tenant data isolation in ui and control-plane-ui is guaranteed by RLS, not only app filters.
- All tenant operations execute with request-scoped allowed organization context.
- Cross-tenant access tests fail as expected across both applications.
- Drizzle remains the single source for policy/role declarations and generated migrations.
