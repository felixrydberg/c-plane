# Billing/Subscription/Payment Exploration Results

## Current Implementation Status

### What EXISTS:
1. **Polar Integration Foundation**
   - `@polar-sh/nuxt` (v0.5.5) & `@polar-sh/sdk` (v0.46.3) in packages/ui & control-plane-ui
   - Polar customer ID stored in `organization.polar_customer_id` (UUID field, unique)
   - Organization deletion triggers Polar customer deletion via `await polar.customers.delete()`

2. **Database Schema (Partial)**
   - `organization` table with `polar_customer_id` field
   - `organization_member` table (members, roles)
   - `organization_invitation` table
   - `active_organization` table (user's active org)
   - `api_keys` table (with scopes)
   - **NO subscription/billing tables yet**

3. **UI Placeholders**
   - "Billing" link in auth layout (packages/ui/app/layouts/auth.vue)
   - Organization type has optional `subscription` property (polar_subscription_id, status, timestamps)
   - Studio admin shows "Polar Customer" ID field

4. **API Endpoints**
   - POST `/api/organization` - creates org with `polar_customer_id: uuidv7()`
   - DELETE `/api/organization/[organization_id]` - deletes Polar customer

### What's MISSING:
1. **Polar Client Configuration**
   - `polar` object referenced in delete.ts but NEVER initialized/imported
   - No Polar API key in .env.example
   - No Polar client setup in utils/auth.ts or elsewhere

2. **Subscription Tracking**
   - No `subscriptions` or `organization_subscriptions` table
   - No DB migration for billing tables
   - Type definition exists but no backing DB model

3. **Usage/Quota Enforcement**
   - No usage tracking tables or models
   - No member limits enforcement
   - No quota validation logic
   - No metering/analytics tables

4. **Billing Routes**
   - No webhook endpoint for Polar (no `/api/webhooks/polar` etc)
   - No `/api/billing` or `/api/subscriptions` endpoints
   - No payment portal/checkout routes

5. **Backend Support**
   - Rust c-plane has no billing dependencies (no stripe/polar crates)
   - No subscription state machine in Rust

6. **Plan Models**
   - No plan definitions (free/pro/enterprise tiers)
   - No capability matrix per plan
   - No feature flags implementation

## Key Findings

### Schema Tables Currently Exist:
- user, account, auth_verification, two_factor
- organization, organization_member, organization_invitation
- active_organization
- api_keys, api_key_scopes
- webhooks, webhook_scopes, webhook_deliveries, webhook_events (studio/better-auth)

### Architecture Issues for Usage-Based Billing:
1. **No metering layer** - can't track API usage, requests, compute time
2. **No audit trail** - no usage events table
3. **Organization limits unenforced** - member count isn't validated
4. **Polar ID collision risk** - `polar_customer_id` is UUID not Polar's string IDs
5. **No subscription status sync** - can't tell if Polar subscription is active
6. **Missing webhook handler** - can't receive Polar payment/subscription events

## Files Referenced:
- [packages/migrations/schema/organization/schema.ts](packages/migrations/schema/organization/schema.ts) - org/member tables
- [packages/ui/shared/types/organization.ts](packages/ui/shared/types/organization.ts) - subscription type def
- [packages/ui/server/api/organization/index.post.ts](packages/ui/server/api/organization/index.post.ts) - org creation
- [packages/ui/server/api/organization/[organization_id]/index.delete.ts](packages/ui/server/api/organization/[organization_id]/index.delete.ts) - Polar deletion
- [packages/ui/nuxt.config.ts](packages/ui/nuxt.config.ts) - Polar module config
- [packages/migrations/drizzle/0000_damp_ultimo.sql](packages/migrations/drizzle/0000_damp_ultimo.sql) - current migrations
