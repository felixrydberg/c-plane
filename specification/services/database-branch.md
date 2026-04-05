---

# 🌿 Postgres Branching — Summary Specification

## 🎯 Purpose

Database branching allows users to create isolated database copies from an existing database state instantly for:

* preview environments
* testing & migrations
* analytics experiments
* per-PR environments

Branching must:

✅ avoid full data copies
✅ be region-scoped
✅ survive cluster loss
✅ integrate with reconciliation model
✅ require no custom Postgres build

---

# 1. Core Concept

A database is treated as **versioned history**, not a disk.

Branching is implemented by:

```text
Backup + WAL history
        ↓
Recover to specific LSN
        ↓
Start new PostgreSQL timeline
```

Each branch becomes an independent database runtime sharing historical data.

---

# 2. Architecture Principles

Branching follows platform rules:

| Platform Rule                | Branch Behavior                   |
| ---------------------------- | --------------------------------- |
| Regions own durability       | branches live in region storage   |
| Clusters disposable          | branches movable between clusters |
| Object storage authoritative | WAL + backups are source of truth |
| Control plane owns state     | branches are metadata resources   |
| Agents reconcile runtime     | CNPG clusters execute branches    |

---

# 3. Resource Model

## Database

Logical database service.

## DatabaseBranch (new resource)

```yaml
DatabaseBranch:
  id
  database_id
  name
  parent_branch
  recovery_lsn
  workspace_id
  region_id
  status
```

A database contains multiple branches:

```text
database
 ├── main
 ├── preview-123
 └── analytics
```

---

# 4. Storage Model

All databases continuously archive:

```text
WAL → regional object storage
Base backups → object storage
```

Storage layout:

```text
region/
  db_<id>/
    branches/
      main/
      preview-123/
```

Each branch uses a **separate archive prefix**.

---

# 5. Branch Creation Flow

### Step 1 — Select Branch Point

Control plane records WAL position:

```sql
SELECT pg_current_wal_lsn();
```

---

### Step 2 — Create Branch Metadata

Control plane creates `DatabaseBranch`.

---

### Step 3 — Reconcile Runtime

Cluster agent creates a new CloudNativePG cluster using recovery bootstrap:

```yaml
bootstrap:
  recovery:
    source: parent-backup
    recoveryTarget:
      targetLSN: <lsn>
```

---

### Step 4 — Promotion (Automatic)

PostgreSQL:

* replays WAL to LSN
* promotes instance
* creates new WAL timeline

Branch becomes independent.

---

# 6. Timeline Isolation

After promotion:

| Component             | Shared |
| --------------------- | ------ |
| history before branch | ✅      |
| future WAL            | ❌      |
| backups               | ❌      |
| compute               | ❌      |

Each branch writes new WAL on its own timeline.

No data corruption possible.

---

# 7. Runtime Model

Each branch runs as:

```text
1 branch = 1 CNPG cluster
```

Compute is independent.

Storage history is shared.

---

# 8. Networking & Access

Each branch receives a stable regional endpoint:

```text
<branch>.db.<region>.platform.dev
```

Routing handled by regional database service layer.

---

# 9. Lifecycle

## Creation

Recovery from backup → new timeline.

## Suspend (optional)

Branch compute may be removed; storage persists.

## Resume

Cluster recreated from branch backups.

## Delete

Remove cluster + archive prefix.

---

# 10. Failure Behavior

| Failure          | Result                 |
| ---------------- | ---------------------- |
| node loss        | branch recreated       |
| cluster loss     | recovered from storage |
| branch isolation | preserved              |

Durability depends only on object storage.

---

# 11. Operational Guarantees

The platform guarantees:

✅ branches are isolated databases
✅ branching does not duplicate full datasets
✅ main database is never modified by branches
✅ branches survive infrastructure replacement

The platform does NOT guarantee:

❌ instant startup (initially)
❌ storage deduplication beyond shared history

---

# 12. Developer Mental Model

User thinks:

```text
Create database branch from main.
```

Platform performs:

```text
Record WAL position
→ Restore backup
→ Replay WAL
→ Promote new timeline
→ Start independent database
```

---

# ✅ One-Sentence Definition

> Database branching is implemented by restoring a database from object-storage backups to a chosen WAL position and promoting it into a new PostgreSQL timeline running as an independent CloudNativePG cluster.

---
