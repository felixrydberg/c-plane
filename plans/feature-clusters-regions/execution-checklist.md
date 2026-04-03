# Execution Checklist

## Phase 1 Contract
- [ ] Define Rust endpoint contract for `/admin/infrastructure/regions` and `/admin/infrastructure/clusters`
- [ ] Define service token and actor header contract
- [ ] Define shared error envelope

## Phase 2 Schema
- [ ] Add `regions` schema folder
- [ ] Add `clusters` schema folder
- [ ] Add `infrastructure-capabilities` schema folder
- [ ] Export new schema modules in `schema/index.ts`
- [ ] Generate migration

## Phase 3 Rust API
- [ ] Add admin infrastructure routes
- [ ] Add handlers/services for regions CRUD
- [ ] Add handlers/services for clusters CRUD
- [ ] Add/align infrastructure data module boundaries (toward InfrastructureRepository)
- [ ] Add service-token middleware + admin actor enforcement

## Phase 4 Studio Proxy
- [ ] Add studio proxy utility for Rust forwarding
- [ ] Add `requireAdmin` helper in studio server authorization
- [ ] Add `server/api/admin/regions/*` proxy routes
- [ ] Add `server/api/admin/clusters/*` proxy routes
- [ ] Add consistent Rust-to-Nuxt error mapping

## Phase 5 Verification
- [ ] Rust auth failure tests (missing token, invalid token, non-admin actor)
- [ ] Rust regions/clusters CRUD tests
- [ ] Studio proxy tests (method/query/body/header passthrough)
- [ ] End-to-end manual validation
