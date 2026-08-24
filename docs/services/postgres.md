# Postgres

## Decision

Postgres is managed by [CloudNativePG](https://cloudnative-pg.io/). C-Plane
renders CloudNativePG resources directly; it does not add another Kubernetes
operator around CloudNativePG.

Every `postgres_database_branch` is one independent CloudNativePG `Cluster`.
The project branch selects the cluster and credentials used by its workloads.

- Standard branches run one Postgres instance.
- High-availability branches run one primary and the requested replicas.
- Compute stays running. CPU and memory may be reduced or increased through
  the branch configuration.
- Scale-to-zero and wake-on-connection are out of scope.

## Resource Model

- `Database` — the logical database owned by an organization, project, and
  region.
- `DatabaseBranch` — the isolated database attached to one project branch. It
  owns compute, HA, backup retention, and runtime state.
- `DatabasePlacement` — the internal association between a database branch and
  the Kubernetes cluster currently running it.
- `DatabaseStorage` — the regional object-store location, backup prefix, and
  recovery-window policy.

Users choose a region, not a Kubernetes cluster. Placement remains internal.
Database backup storage is accessed only through the regional Storage API.

## Durability

- Local PVCs contain the active Postgres data directory.
- CloudNativePG replicas handle pod and node failures inside one Kubernetes
  cluster.
- The Barman Cloud Plugin continuously archives WAL and takes physical base
  backups through the regional Storage API, which owns provider access.
- A whole-cluster loss is recovered by creating a new CloudNativePG cluster
  from the latest base backup and replaying archived WAL.
- A planned cluster removal must migrate and promote a caught-up replica before
  the source is deleted.

The platform requires Kubernetes storage and an S3-compatible endpoint. It
does not require Ceph or another platform-owned storage cluster.

## Backup Retention

Backup retention is a recovery window, not a raw count of WAL files. The Barman
Cloud Plugin retains the base backup and WAL needed to recover to any point
inside that window.

Retention is configurable per database branch because every branch is an
independent CloudNativePG cluster and has its own object-store configuration
and prefix. Main branches receive a durable default. Short-lived preview
branches may use a shorter window or disable independent backups when they can
be recreated from their parent.

Postgres `wal_keep_size` and replication-slot retention are internal replication
settings. They are not the user-facing backup-retention control.

## Backup Storage Isolation

Each region has one physical Postgres backup bucket and one backing-provider
credential. The Storage API alone holds that credential. Every database branch
uses a derived Storage API credential restricted to its immutable prefix, while
Storage encrypts its objects with the owning organization's regional SSE-C key.

This avoids per-branch buckets and OpenBao access-key entries without exposing
one organization's backups to another branch or organization. See [Postgres
Backup Storage Isolation](postgres-backup-storage.md) for credential derivation,
encryption, threat boundaries, and required Storage API enforcement.

## Branch Creation

C-Plane explicitly chooses the fastest valid source. CloudNativePG does not
automatically reuse the parent cluster's volume.

1. **CSI snapshot** — when source and destination use a snapshot-capable storage
   backend, create an on-demand `VolumeSnapshot` backup and bootstrap the new
   cluster from it.
2. **Live clone** — when the source is healthy and snapshots are unavailable,
   bootstrap with `pg_basebackup` over streaming replication.
3. **Object-store recovery** — when the source is unavailable, on another
   incompatible storage backend, or a historical recovery target is requested,
   restore a base backup and replay WAL from S3.

The target always receives new writable volumes and a new backup prefix. A
source PVC must never be mounted by two writable Postgres clusters. The storage
driver decides whether a snapshot restore is an instant copy-on-write clone or
a physical copy.

Preview branches start with one instance so they become available as soon as
the restored primary is healthy. Replicas are added afterward when HA is
requested.

## Recovery and Relocation

Recovery uses the shortest available path:

| Failure or operation | Source |
|---|---|
| Pod or node failure | Existing CloudNativePG replica |
| Planned cluster relocation | Streaming replica cluster, then promotion |
| Branch on compatible storage | CSI volume snapshot |
| Branch without snapshots | Live `pg_basebackup` |
| Whole-cluster loss | S3 base backup and archived WAL |
| Historical branch or restore | Base backup or snapshot plus archived WAL |

Automatic recovery from whole-cluster loss is introduced only after restore and
cutover drills are reliable. Initial disaster recovery may be runbook-driven.

## Lifecycle

`pending → provisioning → restoring → healthy → scaling → draining → failed → deleted`

Deletion removes the CloudNativePG resources and branch-specific snapshots.
While a branch is active, Barman enforces its recovery window. Branch deletion
schedules a durable C-Plane cleanup job for the branch's unique object-store
prefix after the configured retention period; that cleanup must not depend on
the database pods still existing.

## Networking

- Users connect with a standard PostgreSQL connection string over TLS.
- Public databases use a Cilium Gateway `TCPRoute`.
- Private databases expose only a ClusterIP service through the regional mesh.
- The stable product endpoint is independent of the current placement and is
  switched only after the destination is healthy.
- C-Plane provisions branch-scoped credentials. Restored branches rotate their
  application credentials before becoming reachable.

## Ownership

The control plane owns database metadata, placement, source selection,
credentials, retention policy, lifecycle transitions, and endpoint cutover.

The render pipeline emits CloudNativePG, backup, snapshot, service, policy, and
route resources. The cluster agent applies that desired state and reports
status. CloudNativePG owns Postgres reconciliation, replication, failover,
backup execution, and recovery inside the target Kubernetes cluster.

See [Postgres implementation guide](postgres-implementation.md) for the resource
mapping and workflows.
