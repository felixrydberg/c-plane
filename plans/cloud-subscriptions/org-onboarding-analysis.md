# Organization Onboarding & Limits Analysis

## KEY FINDINGS

### Design Documents (Untitled-1 & Untitled-2)
- **Untitled-1**: Defines "Organization Onboarding — Data Retention & State Model"
  - Onboarding should be persistent org metadata (not UI state)
  - Lifecycle states: created -> profile_complete -> environment_ready -> billing_required -> active
  - Must support resumability after interruptions
  - Separate from billing/limits (cloud vs self-hosted)

- **Untitled-2**: Defines "Workspace Limits & Capability Model"
  - Plans assign capabilities/limits, not plan type directly
  - Limits: quantitative (services, projects, members) + features (audit_logs, priority_support)
  - Data-driven, adjustable without redeployment
  - Self-hosted defaults to unlimited, cloud has subscription-based limits

### IMPLEMENTATION GAP - Current Schema Missing Fields

#### [ui-shared/schema/organization/schema.ts](packages/ui-shared/schema/organization/schema.ts)
Organization table ONLY has:
- id, name, email, slug, logo, created_at, **polar_customer_id**

**MISSING**:
- onboarding_status / setup_state field
- capabilities / limits JSON field
- features boolean flags
- Any lifecycle tracking

#### [c-plane/src/models/entities/organisation.rs](c-plane/src/models/entities/organisation.rs)
Model only has:
- id, name, description, is_active, created_at, updated_at, created_by

**MISSING**:
- Any onboarding state
- Any limits/capabilities

### Current Routing/Auth Structure

#### [ui/server/utils/authorization.ts](packages/ui/server/utils/authorization.ts)
- `getOrganizationMembership()`: Only checks user is org member (role-based)
- NO enforcement of organization readiness/onboarding state
- NO limit enforcement checks

#### [ui/app/middleware/auth.global.ts](packages/ui/app/middleware/auth.global.ts)
- Redirects unauthenticated users to `/auth/signin`
- Redirects authenticated users WITHOUT org to `/onboarding`
- **Problem**: No way to track if org is "onboarded" vs "creating"
- **Problem**: No enforcement that only active organizations are accessible

#### [ui/server/api/organization/index.post.ts](packages/ui/server/api/organization/index.post.ts)
- Creates org with: name, email, slug, polar_customer_id (UUID)
- Immediately creates owner membership and sets active_organization
- **Missing**: Any onboarding state initialization
- **Missing**: Any capability/limit assignment
- **Missing**: Any billing state tracking

### Enforcement Gaps
1. **No onboarding state validation**: Any org can be used immediately after creation
2. **No capability enforcement**: When creating services/projects, no checks for org limits
3. **No billing sync**: polar_customer_id stored but no integration with subscription state
4. **No setup completion logic**: No endpoint/service tracks which setup steps were done

## Backend Models & Error Types

### [ui/shared/types/organization.ts](packages/ui/shared/types/organization.ts)
TypeScript Organization type ONLY includes:
- id, name, slug, created_at, logo, member (user role)
- **subscription** (optional): with Polar subscription data (id, status)

**Missing**:
- onboarding_status / setup_state
- capabilities / limits object

### [c-plane/src/models/entities/project.rs](c-plane/src/models/entities/project.rs)
Project model has: id, name, description, organisation_id, owner_id, is_archived, timestamps

### Error Types - [c-plane/src/errors/project.rs](c-plane/src/errors/project.rs)
**Partially Implemented Limit Enforcement:**
- `ProjectLimitExceeded { current: u32, limit: u32 }` error defined
- **ERROR IS DEFINED BUT NOT USED** - No service/handler throws it yet
- No project creation handler exists (routes/services/handlers not implemented)

### Routes Status - [c-plane/src/routes/mod.rs](c-plane/src/routes/mod.rs)
- Only `/health` endpoint is active
- All project-related routes are **commented out**
- Protected routes infrastructure commented out

## Summary of Missing Pieces

### Schema Migrations Needed
1. Add to `organization` table:
   - `onboarding_status` (enum: created|profile_complete|environment_ready|billing_required|active)
   - `setup_metadata` / `provisioning_results` (JSON)
   - `capabilities` (JSON: limits + features)

2. Add to support limits:
   - `limits_json` (quantitative: services, projects, members)
   - `features_json` (boolean flags: audit_logs, priority_support)

### Routes/Handlers Not Yet Implemented
- Project CRUD (create, get, update, delete)
- Organization setup completion endpoint
- Onboarding state transition endpoint
- Capability/limits retrieval endpoint
- Limit enforcement checks in create operations

### Service Layer Gaps
- No project creation service with limit checking
- No onboarding state machine implementation
- No capability assignment logic
- No subscription sync with Polar billing

### Middleware/Authorization Gaps
- [ui/server/utils/authorization.ts](packages/ui/server/utils/authorization.ts):
  - Only checks membership, no onboarding state validation
  - No limit checking before resource creation

### UI Integration Points
- Auth middleware redirects to `/onboarding` but doesn't validate org is actually ready
- Organization create flow doesn't initialize onboarding state
- No feedback on onboarding progress persistence
