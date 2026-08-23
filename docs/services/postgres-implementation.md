# Postgres Implementation Guide

This guide implements the decisions in [Postgres](postgres.md) using
CloudNativePG and the Barman Cloud Plugin.

## 1. Install and Pin Dependencies

Add CloudNativePG and the Barman Cloud Plugin to the required operator manifest.
Pin mutually compatible versions to the C-Plane release and upgrade them through
the existing cluster-agent operator lifecycle.

Each ready database-capable cluster must report:

- CloudNativePG operator version and health.
- Barman Cloud Plugin version and health.
- Default `StorageClass`.
- CSI volume-snapshot support and `VolumeSnapshotClass`, when available.
- Reachability of the regional Storage API endpoint.

Do not infer snapshot support from a provider name. Treat it as a reported
cluster capability.

## 2. Persist the Required Intent

The database branch is the unit of runtime and backup configuration. Persist at
least:

- desired CPU and memory;
- HA enabled and replica count;
- backup recovery-window duration, or backups disabled;
- current placement and lifecycle state;
- immutable CloudNativePG cluster name and backup prefix;
- source branch and requested recovery target during branch creation.

Validate that durable/main branches cannot disable backups. Preview branches
may disable them only when product policy permits recreating the branch.

Never store database passwords, derived Storage secret values, or SSE-C keys in
these records. Store application-secret and encryption-key references where
required. The branch's backup access key is derived from its immutable identity
and credential generation as described in [Postgres Backup Storage
Isolation](postgres-backup-storage.md).

## 3. Render One Database Branch

Render the following resources for each placed database branch:

1. CloudNativePG `Cluster`.
2. Barman `ObjectStore` targeting the regional Storage API with the branch's
   scoped backup prefix, service credential, and `.spec.retentionPolicy`
   recovery window.
3. `ScheduledBackup` using `method: plugin` and the Barman Cloud plugin for
   physical base backups when backups are enabled.
4. Application and replication credential references.
5. `Service`, network policy, and optional Gateway API `TCPRoute`.
6. `Backup` and `VolumeSnapshot` resources only while an operation requires
   them.

Map branch settings as follows:

| C-Plane setting | CloudNativePG intent |
|---|---|
| Standard | `instances: 1` |
| HA with N replicas | `instances: N + 1` |
| CPU/RAM | Pod resource requests and limits |
| Backup retention | Barman recovery-window retention policy |
| Branch-scoped Storage credential | Barman `ObjectStore` endpoint, prefix, and credentials |
| Public | Stable service plus `TCPRoute` |
| Private | ClusterIP service only |

Each region has one Postgres backup Storage bucket. Give every database branch
an opaque prefix within it, such as
`postgres/{organization_id}/{database_id}/{database_branch_id}/`, and derive one
credential restricted to that prefix. Inject only that access-key pair and the
internal Storage API endpoint into Barman; never inject regional provider
credentials or an SSE-C key. Do not persist the derived secret in OpenBao.
Storage resolves the credential's organization-and-region encryption-key
reference itself. Set the immutable branch cluster name through the Barman plugin's
`serverName` parameter when reading a source; do not set the compatibility-only
`serverName` field in the `ObjectStore` configuration.

Some CNPG integrations require Kubernetes `Secret` references. Where workload
identity is unavailable, the control plane issues the derived branch pair and
the cluster agent materializes the minimum operator-scoped Kubernetes `Secret`.
Exclude its value from control-plane state, logs, and rendered payload history.

## 4. Create a Main Branch

1. Allocate an immutable database-branch ID, CloudNativePG name, backup prefix,
   and credential generation.
2. Select a healthy database-capable cluster in the requested region.
3. Derive the Storage credential, then render `ObjectStore`, its secret,
   `ScheduledBackup`, and an `initdb`
   CloudNativePG `Cluster`.
4. Wait for the primary and requested replicas to become healthy.
5. Verify WAL archiving before reporting a durable branch healthy.
6. Publish the stable endpoint.

## 5. Create a Child Branch

Resolve the source in this order:

```text
same cluster and CSI snapshots available → volume snapshot
source healthy and reachable             → pg_basebackup
otherwise                                → object-store recovery
```

### CSI Snapshot Path

1. Request an on-demand CloudNativePG `Backup` using the `volumeSnapshot`
   method, preferably from a standby.
2. Wait until the backup and all required snapshots are complete.
3. Render the target CloudNativePG `Cluster` with
   `bootstrap.recovery.volumeSnapshots` referencing those snapshots.
4. Start the target with one instance.
5. Rotate application credentials and verify branch isolation.
6. Publish the endpoint, then add replicas if HA was requested.

Use a coordinated hot snapshot for an online source. Keep WAL archiving enabled
so the target can replay to the required recovery point. A cold snapshot is
acceptable only when fencing the selected source instance is safe.

### Live Clone Path

1. Expose the source through a private replication endpoint.
2. Render it as an `externalClusters` streaming source for the target.
3. Bootstrap the target with `pg_basebackup`.
4. Keep the target independent; do not configure replica mode for a branch
   clone.
5. Rotate credentials, configure the target's own backup prefix and scoped
   Storage credential, and publish the endpoint.

The source and target Postgres major versions and physical layout must be
compatible.

### Object-Store Path

1. Issue a temporary derived read-only credential for the source prefix and
   reference its Storage-backed Barman object store as an external cluster.
2. Render `bootstrap.recovery` with the latest target or an explicit timestamp
   or LSN.
3. Restore the selected base backup and replay archived WAL.
4. Rotate credentials and switch the target to its own backup prefix before
   publishing the endpoint.

## 6. Configure Retention

Expose a recovery-window duration, not `wal_keep_size` and not a raw WAL-file
count. The Barman policy must retain the base backup immediately before the
window plus all WAL needed through the newest archived point.

Suggested product defaults:

| Branch class | Recovery window |
|---|---|
| Durable/main | 30 days |
| Staging | 7 days |
| Preview | 1 day or disabled |

Keep CSI snapshot cleanup separate from Barman retention. Delete operation-only
snapshots after the branch is healthy unless a snapshot retention feature is
explicitly enabled.

Storage lifecycle policy must expire objects later than Barman does; it must
never delete WAL that remains inside the configured recovery window.

Barman enforces retention only while its plugin is running. When a branch is
deleted, C-Plane must retain a durable cleanup job that deletes the branch's
unique prefix after the selected retention or deletion grace period. Bucket
lifecycle is only a longer-lived safety net.

## 7. Scale Compute

Update CloudNativePG pod resources from the desired branch CPU and memory.
CloudNativePG owns the resulting rollout. Keep the stable service in place and
report `scaling` until the requested instances are healthy.

Do not implement automatic suspension. Autoscaling policy, if added later,
belongs in the C-Plane scheduler and writes the same desired resource fields.

## 8. Planned Cluster Relocation

1. Freeze new scheduling onto the source cluster.
2. Render the destination as a CNPG replica cluster. Bootstrap it from a
   snapshot, `pg_basebackup`, or the latest Storage backup; CNPG replays
   archived WAL, then continuously streams any remaining WAL from the source.
   Keep the Storage WAL archive configured as the streaming fallback.
3. Automatically observe replication lag. At the cutover threshold, stop or
   fence source writes, wait for the destination to reach the final source LSN,
   then promote it.
4. Verify write health, switch the stable endpoint, and observe the destination
   before deleting the source placement.

Never delete the source cluster merely because the destination pods exist.

## 9. Unplanned Recovery

1. Fence the failed placement in control-plane state so it cannot rejoin as a
   writer.
2. Select a healthy destination cluster in the same region.
3. Restore the latest valid base backup through Storage and replay all available
   WAL.
4. Create requested replicas after the recovered primary is healthy.
5. Verify the recovery target, rotate credentials when required, and switch the
   endpoint.
6. Preserve the failed placement for investigation until split-brain is ruled
   out.

Use parallel object download and WAL restore supported by the pinned Barman
Plugin version. Recovery is not complete until applications can read and write
through the stable endpoint.

## 10. Delete a Branch

1. Mark the branch `deleting` and stop new connections.
2. Remove routes and application credential access.
3. Record when its branch-unique backup prefix becomes eligible for deletion.
4. Delete the CloudNativePG cluster and operation-only snapshots.
5. Create or retain a durable C-Plane cleanup job that purges the prefix after
   the retention or deletion grace period. Run it immediately only when
   permanent deletion was explicitly requested and authorized.
6. Remove placement metadata after Kubernetes cleanup is observed, but keep the
   cleanup job until object-store deletion is confirmed.

Deleting a project branch must never delete snapshots or backups still used by
another database branch.

## 11. Verification

Before enabling production automation, keep one runnable integration path that
proves:

- Main creation and WAL archiving.
- Branch creation through snapshot and `pg_basebackup` fallback.
- PITR to a known transaction.
- Single-instance failure and CNPG failover.
- Planned cross-cluster promotion without split-brain.
- Full restore after deleting the source Kubernetes cluster.
- Retention cleanup without removing the oldest still-required base backup or
  WAL.

Record database size, snapshot duration, restore throughput, WAL archive lag,
replication lag, and endpoint cutover time.

## References

- [CloudNativePG bootstrap](https://cloudnative-pg.io/docs/1.30/bootstrap/)
- [CloudNativePG recovery](https://cloudnative-pg.io/docs/1.30/recovery/)
- [CloudNativePG replica clusters](https://cloudnative-pg.io/docs/1.30/replica_cluster/)
- [Barman Cloud Plugin usage](https://cloudnative-pg.io/plugin-barman-cloud/docs/usage/)
- [Barman retention policies](https://cloudnative-pg.io/plugin-barman-cloud/docs/retention/)
