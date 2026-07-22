# Runtime/Deployment Models Exploration Report

## Summary
**Current Status**: Service/deployment runtime models **NOT YET IMPLEMENTED** in the codebase.

The c-plane project currently defines organizational, user, and API management structures but lacks explicit runtime service deployment models with CPU, RAM, storage, and deployment status definitions.

---

## Current Data Models

### Rust Backend (`packages/c-plane/src/`)
**Location**: [c-plane/src/models/entities/](c-plane/src/models/entities/)

#### Defined Entities:
1. **Organisation** ([organisation.rs](c-plane/src/models/entities/organisation.rs#L1))
   - Fields: `id`, `name`, `description`, `is_active`, `created_at`, `updated_at`, `created_by`
   - No runtime/resource tracking

2. **OrganisationMember** ([organisation_member.rs](c-plane/src/models/entities/organisation_member.rs#L1))
   - Fields: `id`, `organisation_id`, `identity_id`, `role`, `is_active`, `joined_at`, `invited_by`, `invited_at`

3. **Project** ([project.rs](c-plane/src/models/entities/project.rs#L1))
   - Fields: `id`, `name`, `description`, `organisation_id`, `owner_id`, `is_archived`, `created_at`, `updated_at`
   - No deployment/runtime configuration

### TypeScript/Nuxt Schemas (`packages/ui-shared/schema/`)
**Database Location**: [ui-shared/drizzle/0000_damp_ultimo.sql](ui-shared/drizzle/0000_damp_ultimo.sql)

#### Defined Tables:
1. **organization** - Basic org metadata (name, email, slug, logo, Polar integration)
2. **organization_member** - Member roles and membership
3. **organization_invitation** - Invitation workflow (pending/accepted/declined/revoked)
4. **active_organization** - User's active org tracking
5. **api_keys** - API authentication (id, name, key_hash, expires_at, allowed_ips)
6. **api_key_scopes** - API key permission scopes
7. **webhooks** - Webhook definitions (name, url, secret, enabled)
8. **webhook_scopes** - Events: verification:created, completed, revoked
9. **webhook_deliveries** - Event delivery tracking (payload, retry_count, completed)
10. **user**, **account**, **auth_verification**, **two_factor** - Better Auth

---

## Missing Runtime Models

### NOT DEFINED (Required for service deployment):
- Service/Deployment table: deployment_id, service_id, deployment_type, status
- Resource Allocation: cpu_cores, memory_mb, storage_gb, disk_type
- Runtime Status: running, stopped, failed, pending, healthy, unhealthy
- Runtime Metrics: uptime, cpu_usage, memory_usage, disk_usage
- Environment Config: environment_variables, secrets, configuration values
- Deployment History: deployment_logs, rollback_info, version tracking
- Health Checks: health_check_status, last_check_time, failure_count

---

## Telemetry/Observability Status

### Rust Backend (`c-plane/`)
**Files Checked**: [main.rs](c-plane/src/main.rs) [config/mod.rs](c-plane/src/config/mod.rs) [state/mod.rs](c-plane/src/state/mod.rs)

**Current**:
- Basic database logging: `sqlx_logging(true)` in [state/mod.rs](c-plane/src/state/mod.rs#L15)
- No tracing/instrumentation crates configured (despite dependencies in Cargo.lock)
- No prometheus metrics setup
- No structured logging framework

**Dependencies in Cargo.lock**:
- `tracing` listed but not actively used
- No `prometheus`, `opentelemetry`, or `jaeger` crates in main package

### Nuxt Apps (`packages/ui/`, `packages/control-plane-ui/`)
**Telemetry Status**:
- `disable-telemetry` flag in hero components ([ui/hero.vue#L8](ui/app/components/auth/hero.vue#L8), [control-plane-ui/hero.vue#L8](control-plane-ui/app/components/auth/hero.vue#L8))
- No custom metrics/telemetry ingestion implemented
- No @nuxt/telemetry configured for custom events

---

## Key Configuration Files

### Rust
- [Cargo.toml](c-plane/Cargo.toml) - Dependencies (axum, sea-orm, tokio, uuid, chrono, ory-client)
- [config/mod.rs](c-plane/src/config/mod.rs) - Only loads DATABASE_URL, KRATOS_API_KEY, SERVER_HOST/PORT
- [main.rs](c-plane/src/main.rs) - Minimal setup, binds to 0.0.0.0:8080

### Nuxt
- [ui/nuxt.config.ts](ui/nuxt.config.ts#L50) - Experimental WebSocket & OpenAPI
- [control-plane-ui/nuxt.config.ts](control-plane-ui/nuxt.config.ts#L50) - Same config plus HMR on port 24679

---

## Deployment Context

### Docker Setup
- [Dockerfile.rust](c-plane/Dockerfile) - Multi-stage build with non-root user
- HEALTHCHECK: `curl -f http://localhost:8080/health || exit 1`
- [docker-compose.dev.yml](docker-compose.dev.yml) - Dev environment with ui, control-plane-ui, c-plane services

### Environment Variables in Compose
- `NUXT_DATABASE_URL` - PostgreSQL connection
- `NUXT_REDIS_URL` - Redis for sessions
- `BETTER_AUTH_SECRET` - Authentication
- No service-specific runtime config

---

## Action Items for Implementation

To add service/deployment runtime tracking, create:

1. **Database Schema** (`packages/ui-shared/schema/service/schema.ts`):
   - deployments table (id, organization_id, service_id, type, status, created_at, updated_at)
   - service_resources (id, deployment_id, cpu_cores, memory_mb, storage_gb, gpu_enabled)
   - service_health (id, deployment_id, status, cpu_usage, memory_usage, last_check)

2. **Rust Models** (`packages/c-plane/src/models/entities/`):
   - deployment.rs
   - service_resource.rs
   - health_status.rs

3. **Observability** (both backends):
   - Add OpenTelemetry/Prometheus collectors
   - Structured logging with context propagation
   - Metrics export endpoints

4. **Nuxt API Routes** (`packages/ui/server/api/`):
   - /deployments/* endpoints
   - /services/* endpoints
   - /metrics/* endpoints

---

## Files of Interest

| Type | Path | Status |
|------|------|--------|
| Schema Index | [ui-shared/schema/index.ts](ui-shared/schema/index.ts) | Re-exports only org, api-keys, auth, webhooks |
| Entity Index | [c-plane/src/models/entities/mod.rs](c-plane/src/models/entities/mod.rs) | Exports: org, member, project |
| API Routes | [ui/server/api/organization/](ui/server/api/organization/) | Organization management only |
| Config | [c-plane/src/config/mod.rs](c-plane/src/config/mod.rs) | No service runtime config |
| Migrations | [ui-shared/drizzle/0000_damp_ultimo.sql](ui-shared/drizzle/0000_damp_ultimo.sql) | No service/deployment tables |
