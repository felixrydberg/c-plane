# C-Plane

## Style guides

Before writing any UI code, read these:

- [Button style guide](docs/ui/buttons.md) — solid colors, text+icon, loading states

## Architecture

- `packages/c-plane` — Rust backend (axum, sea-orm, utoipa)
- `packages/ui` — Nuxt frontend
- `packages/migrations` — Drizzle schemas, migrations, and database types
- `packages/lib` — reusable Rust services, currently OpenBao secrets

## Patterns

- Error handling: prefer `AppError::NotFound` / `AppError::Conflict` over custom error variants. Add new variants only when existing ones don't cover the case.
- API: every handler takes `AuthContext` (tenant_db resolves org access). Verify org access first, then project-in-org, then operate.
- OpenAPI: every Rust route must have a matching `utoipa::path` entry and all request/response types exposed by that route must be registered in `packages/api/src/openapi.rs`.
- Frontend: use `useFetch` / `await useFetch` during SSR to avoid hydration mismatches. Store projects in `store.projects` — they're loaded by the auth plugin before any page renders.
