# Cluster Draining & Persistence — Refined Decisions

## Core Principle

- Clusters are disposable compute. Persistence must outlive clusters.
- A cluster can be destroyed without data loss if storage rules are followed.

## Storage Classes

### Ephemeral (default)
- Lives on node disk, tied to pod lifecycle.
- Deleted on reschedule or cluster drain.
- No platform guarantees. Used for temp files, caches, build artifacts.
- Equivalent to Kubernetes `emptyDir`.

### Durable Object Storage (primary persistence)
- All real persistence goes through S3-compatible storage.
- Survives cluster replacement, works across clusters, region-shareable.
- Region durability guarantee: objects stored in region storage survive cluster loss.
- Supported backends: AWS S3, Cloudflare R2, Backblaze B2, MinIO, Ceph RGW (Rook), any S3-compatible system.
- Rook/Ceph is optional for self-hosters, not required.

### Persistent Volumes (specialised)
- Used for database working storage and high IOPS workloads (e.g. CloudNativePG data directory).
- Treated as performance caches, not the source of truth.
- Durability comes from replication, WAL shipping, and S3 backups — not from the volume itself.

## Database Persistence Model

- Local PVC for working storage.
- CloudNativePG replication handles pod and node failure inside a cluster.
- Continuous WAL backup to S3.
- Planned cluster removal creates and promotes a caught-up destination replica
  before deleting the source.
- On unplanned cluster loss, a new cluster restores from S3, replays WAL, and
  resumes service.

## Ephemeral Data Loss (e.g. in-progress jobs)

- Acceptable by design. Workloads using ephemeral storage must be restartable or checkpointable.
- Mitigations: checkpoint progress to S3, stream output directly to object storage, retry semantics.
- Graceful drain window allows in-progress work to finish before eviction.

## Drain Sequence

1. Scheduling freeze — no new workloads assigned to the cluster.
2. Workload migration — stateless pods rescheduled, jobs retried, destination database replicas created and caught up.
3. Storage handling — databases promoted and endpoints switched before source PVCs are deleted; object storage remains untouched.
4. Cluster removal — safe because all durable state is external.

## Key Rules

- Clusters are disposable.
- Object storage is durable.
- PVCs are recoverable working copies, not authoritative state.
- Ephemeral storage may vanish at any time.
- Databases must continuously back up to object storage.
- The platform requires an S3 endpoint, not a storage cluster.
