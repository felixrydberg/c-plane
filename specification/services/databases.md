
---

# 🐘 Postgres Deployment Specification

## 1. Goal

Provide **managed PostgreSQL databases** as a platform primitive where:

✅ users deploy databases to **regions**, not clusters
✅ clusters remain replaceable compute
✅ durability survives cluster loss
✅ databases integrate with workspace isolation
✅ operations are automated by control plane reconciliation
✅ self-host and cloud behave identically

The platform exposes:

> **Postgres-as-a-Regional-Service**

---

# 2. Core Design Principles

## 2.1 Region-Owned Databases

Databases belong to **regions**, never clusters.

```
User → Region → Database → Runtime Placement → Cluster
```

Clusters only run replicas.

This matches your deployment model where regions define desired state .

---

## 2.2 Durability Outside the Cluster

Clusters are disposable.

Database durability comes from:

```
Replication
+ WAL shipping
+ Continuous backups → S3
```

PVCs are working storage only .

---

## 2.3 Control Plane Is Source of Truth

Kubernetes is execution only.

```
Control Plane DB
      ↓ desired state
Cluster Agent
      ↓ reconcile
CloudNativePG resources
```

Agents never make scheduling decisions .

---

## 2.4 Workspace Isolation

Each database belongs to a workspace.

Authorization integrates with your RLS identity model:

```
Identity → Workspace → Database Access
```

Platform auth governs access, not Kubernetes.

---

# 3. Resource Model

## 3.1 Database Object

A Postgres deployment is a **platform resource**.

### Database

```
Database
  id
  workspace_id
  region_id
  name
  version
  size_class
  high_availability_mode
  status
```

Represents logical database service.

---

## 3.2 Runtime Placement (Internal)

```
DatabasePlacement
  database_id
  cluster_id
  role (primary | replica)
  status
```

Users never see this.

---

## 3.3 Storage Configuration

```
DatabaseStorage
  backup_bucket
  wal_prefix
  retention_policy
```

Always region-scoped object storage.

---

# 4. Deployment Lifecycle

## 4.1 Create Database

User action:

```
Create database
Region: eu-west
Size: small
```

---

### Control Plane Flow

```
1. Validate workspace limits
2. Select region
3. Create Database record
4. Scheduler chooses cluster(s)
5. Desired state emitted
```

---

### Agent Reconciliation

Agent creates:

* CloudNativePG cluster
* PVC
* backup configuration
* replication setup

---

## 4.2 Database States

```
pending
→ provisioning
→ restoring (optional)
→ healthy
→ scaling
→ draining
→ failed
→ deleted
```

Driven by reconciliation loops.

---

# 5. High Availability Model

Two supported modes:

---

## 5.1 Single Primary (Reactive)

```
Primary on Cluster A
Backups → S3
```

Failure:

```
Cluster dies
→ restore from backup
→ new primary created
```

Simplest self-host mode.

---

## 5.2 Regional HA (Preferred)

```
Primary: Cluster A
Replica: Cluster B
Replica: Cluster C
```

Streaming replication.

Cluster failure:

```
Replica promoted automatically
```

Aligns with regional HA architecture .

---

# 6. Backup & Recovery (MANDATORY)

Every database MUST enable:

### Continuous Backup

```
WAL → Object Storage
```

### Base Backups

Periodic snapshots.

### Restore Capability

User may create:

```
Restore → new database
point-in-time restore
```

Object storage is source of truth .

---

# 7. Networking Model

Users never connect directly to pods.

They connect via **regional database endpoint**:

```
db.<region>.platform.dev
```

Routing handled by:

* regional service abstraction
* Cilium mesh networking 

Endpoint survives cluster migration.

---

# 8. Authentication Model

## Platform Authentication Layer

User connects using platform credentials.

Platform performs:

```
authenticate user
resolve workspace
verify DB access
issue connection credentials
```

---

### Credential Options

#### A — Managed Roles (recommended)

Platform creates DB roles automatically.

```
workspace_app_user
read_only_user
admin_user
```

#### B — Ephemeral Credentials (future)

Short-lived credentials issued per session.

---

# 9. Scaling Operations

## Vertical Scaling

```
Update size_class
→ reconcile
→ rolling restart
```

---

## Horizontal Scaling

Add replicas:

```
desired_replicas = N
```

Scheduler distributes across clusters.

---

# 10. Cluster Drain Behavior

When cluster drains:

| Component | Action                    |
| --------- | ------------------------- |
| replica   | recreate elsewhere        |
| primary   | failover                  |
| PVC       | discarded                 |
| data      | recovered via replication |

No manual migration required.

Consistent with disposable cluster rule .

---

# 11. Control Plane Responsibilities

Control plane owns:

* database metadata
* scheduling decisions
* credential issuance
* backup policies
* lifecycle transitions

---

Clusters own:

* execution only
* reconciliation
* health reporting

Strict boundary preserved .

---

# 12. Developer Mental Model

Users think:

```
I deploy a database to a region.
```

Reality:

```
Control plane schedules replicas
across disposable clusters
backed by regional object storage.
```

---

# 13. Guarantees (Platform Contract)

The platform guarantees:

✅ database survives cluster loss
✅ stable regional endpoint
✅ automated backups
✅ workspace isolation
✅ no Kubernetes knowledge required

The platform does NOT guarantee:

❌ local disk durability
❌ node persistence
❌ zero restart during failover (unless HA enabled)

---

# 14. Minimal Self-Host Requirements

To run Postgres service:

```
Kubernetes cluster
+
S3-compatible storage
```

No Ceph or distributed filesystem required .

---

# 15. Final Architecture Diagram

```
                CONTROL PLANE
                     │
          desired database state
                     │
        ┌────────────┼────────────┐
        │            │            │
     Cluster A    Cluster B    Cluster C
     Primary       Replica       Replica
        │            │            │
        └────────────┼────────────┘
                     │
              Regional Object Storage
                (backups + WAL)
```

---

## ✅ One-Sentence Definition

> A Postgres deployment is a **region-scoped managed database** whose runtime replicas run on disposable clusters while durability is guaranteed through replication and continuous object-storage backups.

---
