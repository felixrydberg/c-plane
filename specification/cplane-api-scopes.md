
---

# 🧠 Implementation Prompt — Rust Multi-Tenant API (RLS + SeaORM)

## System Overview

You are implementing a **Rust backend API** that serves:

1. A **Better Auth–based dashboard frontend**
2. A **public custom API** accessed via API keys

The system uses:

* **Rust**
* **Axum**
* **SeaORM**
* **PostgreSQL**
* **Row Level Security (RLS)**
* **pgBouncer (transaction pooling)**

The architecture follows a **data-scoped permission system** where authorization is enforced primarily at the database layer. 

---

# 🎯 Goals

The API must:

* enforce workspace isolation using PostgreSQL RLS
* allow multiple workspaces simultaneously
* support both user auth and API key auth
* prevent accidental cross-tenant queries
* remain pgBouncer-safe
* keep handlers free from authorization logic
* treat schemas as externally owned by `ui-shared`

---

# 🧱 Repository Structure Assumption

Monorepo layout:

```
repo/
 ├─ ui-shared/        ← SOURCE OF TRUTH (schemas + migrations)
 ├─ cplane/         ← THIS PROJECT
 └─ ui/
```

---

# ⚠️ Schema Ownership Rule (VERY IMPORTANT)

## `ui-shared` owns the database schema.

* All migrations originate from **ui-shared**
* Rust API **does NOT define schema authority**
* SeaORM entities must **mirror**, not generate schema

### Therefore:

✅ SeaORM is used for:

* querying
* relations
* models
* NOT SCHEMA FOR MIGRATIONS. THAT IS ALL HANDLED BY `ui-shared` & DRIZZLE.

---

# 🧩 Database Security Model

## PostgreSQL Roles

Create three DB roles:

| Role           | Purpose             |
| -------------- | ------------------- |
| `app_identity` | identity queries    |
| `app_tenant`   | tenant data via RLS |
| `app_admin`    | migrations          |

---

## Identity Tables

Accessible only by `app_identity`.

```
users
organization
organization_memberships
api_keys
```

Used for:

* authentication
* organization resolution
* API key lookup

---

## Tenant Tables

Contain:

```
organization_id UUID NOT NULL
```

RLS enabled.

Example:

```
projects
tasks
services
```

---

## RLS Policy Pattern

All tenant tables follow:

```sql
USING (
  organization_id = ANY(
    current_setting('app.allowed_organizations')::uuid[]
  )
);
```

---

# 🔐 Authentication Model

## 1️⃣ User Authentication (Better Auth)

Frontend authenticates via Better Auth.

Rust API receives identity information.

User resolution is handled manually.

### Placeholder Required

Implement:

```rust
async fn resolve_user_from_request(...) -> UserContext {
    // TODO: integrate Better Auth verification
}
```

Do NOT implement auth logic yet.

---

## 2️⃣ API Key Authentication

API keys are resolved directly in Postgres.

Table:

```
api_keys
---------
id
hashed_key
workspace_id
permissions
```

Flow:

1. Extract API key from header.
2. Query using `AppDatabase`.
3. Resolve allowed workspace(s).
4. Create `WorkspaceContext`.

---

# 🧠 Database Context Types

## AppDatabase (Identity Scope)

Used before tenant context exists.

```rust
pub struct AppDatabase(DatabaseConnection);
```

Allowed operations:

* resolve API keys
* fetch memberships
* resolve workspaces

Must connect using:

```
app_identity role
```

---

## TenantDatabase (Tenant Scope)

Created per request.

```rust
pub struct TenantDatabase(DatabaseTransaction);
```

Responsibilities:

* begin transaction
* set RLS workspace context
* execute tenant queries

---

### Context Initialization

Must use:

```sql
SET LOCAL app.allowed_organizations = '{uuid1,uuid2}';
```

NEVER use `SET` (pgBouncer unsafe).

---

# 🔁 Request Lifecycle

```
Request
   ↓
Auth middleware
   ↓
Resolve user OR API key
   ↓
Fetch memberships (AppDatabase)
   ↓
Create WorkspaceContext
   ↓
Create TenantDatabase transaction
   ↓
SET LOCAL app.allowed_organizations = '{uuid1,uuid2}';
   ↓
Handler executes SeaORM queries
   ↓
Commit transaction
```

---

# 🧩 Workspace Context

```rust
pub struct WorkspaceContext {
    pub allowed_workspaces: Vec<Uuid>,
    pub actor_id: Uuid,
}
```

Used only to initialize TenantDatabase.

---

# 🧱 Repository Pattern

## Identity Repositories

Use `AppDatabase`.

Examples:

```
UserRepository
WorkspaceRepository
ApiKeyRepository
```

---

## Tenant Repositories

Use `TenantDatabase`.

Examples:

```
ProjectRepository
ServiceRepository
TaskRepository
```

Repositories MUST NOT filter by workspace.

Example:

```rust
Project::find().all(&db).await?;
```

RLS handles filtering.

---

# 🚫 Forbidden Patterns

DO NOT:

* add workspace filters in queries
* pass raw DatabaseConnection into handlers
* bypass TenantDatabase
* implement RBAC in handlers
* create schema migrations from SeaORM entities

---

# ✅ Required Middleware

Implement middleware that:

1. detects auth type (user vs API key)
2. resolves workspace access
3. creates TenantDatabase
4. injects into request extensions

Handlers receive ONLY:

```rust
Extension<TenantDatabase>
```

---

# ⚙️ pgBouncer Requirements

Must support:

```
pool_mode = transaction
```

All tenant access MUST occur inside transactions.

---

# 🧠 Mental Model

```
Identity decides ACCESS
Database enforces ISOLATION
Rust guarantees CORRECT USAGE
```

---

# ✅ Expected Developer Experience

Handlers look like:

```rust
async fn list_projects(
    Extension(db): Extension<TenantDatabase>,
) -> Result<Json<Vec<Project>>, Error> {
    Ok(Json(Project::find().all(&*db).await?))
}
```

No auth logic.

No tenant filtering.

---

# 🧾 Deliverables

Implement:

* database context types
* middleware
* repository layers
* RLS-safe transaction handling
* API key resolution
* Better Auth placeholder resolver
* SeaORM entity mirroring system

---

# ⭐ Success Criteria

The system must guarantee:

* cross-tenant data leaks are impossible without DB misconfiguration
* pgBouncer reuse cannot leak context
* developers cannot accidentally run unscoped queries
* schema authority remains in `ui-shared`

---
