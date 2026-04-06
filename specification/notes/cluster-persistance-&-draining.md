---

# Storage Architecture — Summary

## 1. Core Design Principle

Your platform treats **clusters as disposable compute**, not as storage anchors.

**Persistence must outlive clusters.**

This leads to a strict separation:

| Layer    | Responsibility             |
| -------- | -------------------------- |
| Cluster  | compute + scheduling       |
| Region   | durability boundary        |
| Storage  | externalized persistence   |
| Platform | orchestration & guarantees |

A cluster may disappear without data loss.

---

## 2. Storage Categories (Important Distinction)

The platform defines **three storage classes** with different guarantees.

---

### A. Ephemeral Storage (Default)

**Purpose:** runtime scratch space.

Examples:

* ffmpeg outputs
* temp files
* caches
* build artifacts
* model inference buffers

Characteristics:

* lives on node disk
* tied to pod lifecycle
* not migrated
* may disappear during reschedule
* fastest possible IO

Equivalent to:

* Kubernetes `emptyDir`
* container writable layer

✅ No platform guarantees.

This is the **default storage mode**.

---

### B. Durable Object Storage (Primary Persistence)

**This is the platform’s canonical persistence layer.**

All real persistence goes through **S3-compatible storage**.

Examples:

* database backups
* user uploads
* artifacts
* logs
* datasets
* media
* application state blobs

Key decision:

> The platform itself is S3-native, not volume-native.

---

#### Why S3 as the foundation?

Because it:

* survives cluster replacement
* works across clusters
* is region-shareable
* avoids mandatory infra requirements
* matches cloud-native reality
* simplifies migration

This removes the need for:

❌ mandatory regional storage clusters
❌ Ceph requirement
❌ distributed block replication

---

### Storage Providers (Pluggable)

A region may use:

* AWS S3
* Cloudflare R2
* Backblaze B2
* MinIO
* Ceph RGW (Rook)
* any S3-compatible system

Self-hosters can optionally run:

> Rook → Ceph → S3 gateway

But it is **not required**.

---

### Platform Guarantee

A region guarantees:

> Objects stored in region storage survive cluster loss.

---

### C. Persistent Volumes (Specialized Layer)

Persistent volumes still exist — but are **no longer the source of truth**.

They are treated as:

> Performance caches or database working storage.

Used for:

* CloudNativePG data directory
* stateful workloads needing POSIX FS
* high IOPS workloads

But durability comes from:

* replication
* WAL shipping
* backups to S3

NOT from the volume itself.

---

## 3. Database Persistence Model (CloudNativePG)

Databases follow cloud patterns:

```
Local PVC
    +
Streaming replication
    +
Continuous backup → S3
```

Durability source:

✅ object storage backups
✅ replicas on other clusters

NOT:

❌ single cluster disks

---

### Result

If a cluster dies:

1. New cluster starts
2. Database restores from S3
3. WAL replay
4. Service resumes

Region survives.

---

## 4. Shared Storage Across Clusters

Instead of shared block storage:

You share **object storage**.

```
Cluster A ─┐
Cluster B ─┼── Regional S3
Cluster C ─┘
```

Benefits:

* no distributed filesystem complexity
* no Ceph federation problems
* no split-brain risk
* easy scaling
* works across regions later

---

## 5. Handling Ephemeral Data Loss (ffmpeg/cache problem)

Concern:

> Draining cluster deletes temp files.

Correct — and acceptable.

Because:

### Platform Rule

Ephemeral workloads must be:

* restartable
* checkpointable
* resumable

Mitigations:

| Strategy          | Behavior                         |
| ----------------- | -------------------------------- |
| Job checkpoints   | upload progress to S3            |
| Output streaming  | write directly to object storage |
| Retry semantics   | job resumes elsewhere            |
| Graceful draining | time to finish work              |

Cluster drain becomes:

> performance disruption, not data loss.

---

## 6. Cluster Drain Model

Draining a cluster means:

### Step 1 — Scheduling Freeze

No new workloads scheduled.

---

### Step 2 — Workload Migration

| Workload       | Action                       |
| -------------- | ---------------------------- |
| Stateless pods | reschedule                   |
| Jobs           | retry/resume                 |
| Databases      | failover replica             |
| PVC workloads  | recreate from backup/replica |

---

### Step 3 — Storage Handling

* Ephemeral volumes deleted
* PVCs detached
* Databases promoted elsewhere
* Object storage untouched

---

### Step 4 — Cluster Removal

Safe because persistence is external.

---

## 7. Why NOT Regional Ceph Requirement

Running storage clusters per region would:

* increase operational burden
* hurt self-host adoption
* introduce fragile distributed systems
* require deep storage expertise

You intentionally avoid:

> “platform requires storage cluster”

Instead:

> “platform requires S3 endpoint.”

Much simpler mental model.

---

## 8. Self-Host UX Outcome

### Minimal install requirement

A single cluster can run with:

```
Kubernetes
+
S3 endpoint (external OR local)
```

Optional upgrades:

* Rook/Ceph
* MinIO HA
* cloud provider storage

Progressive complexity.

---

## 9. Architectural Outcome

Your platform becomes:

### Compute Platform

Clusters = replaceable execution cells.

### Storage Platform

S3 = durability layer.

### Region

Logical grouping sharing object storage.

---

## 10. Final Mental Model

```
                REGION
        ┌────────────────────┐
        │   S3 Object Store   │  ← durability
        └─────────┬──────────┘
                  │
     ┌────────────┼────────────┐
     │            │            │
 Cluster A    Cluster B    Cluster C
 (compute)     (compute)     (compute)

 PVCs = working copies
 S3   = source of truth
```

---

## 11. Key Rules (Platform Contract)

1. Clusters are disposable.
2. Object storage is durable.
3. PVCs are recoverable, not authoritative.
4. Ephemeral storage may vanish.
5. Databases must continuously back up to object storage.
6. Region durability = object storage durability.

---
