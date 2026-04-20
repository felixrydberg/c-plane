# Postgres — Refined Decisions

## What a Postgres Database Is

- A region-scoped managed database service.
- Users deploy databases to regions, never to clusters.
- Clusters run the replicas. Clusters are disposable. The database is not.

## Resource Model

- `Database` — logical database resource owned by a workspace and a region.
- `DatabasePlacement` — internal record linking a database to a specific cluster with a role (primary or replica). Never user-visible.
- `DatabaseStorage` — backup bucket, WAL prefix, and retention policy. Always region-scoped object storage.

## Durability Model

- Durability comes from replication + WAL shipping + continuous S3 backups. Not from the PVC.
- PVCs are working storage only. They are discarded on cluster drain.
- If a cluster dies: new cluster restores from S3, replays WAL, resumes service.
- Minimum self-host requirement: Kubernetes + S3-compatible storage. No Ceph required.

## High Availability Modes

- **Single primary (reactive)** — one primary, backups to S3. On failure, restore from backup and start new primary. Simplest mode.
- **Regional HA (preferred, control-plane orchestrated)** — primary in one Kubernetes cluster, CloudNativePG replica cluster(s) in others (distributed topology). On primary cluster failure, control plane promotes a replica cluster and switches routing/endpoint. No full backup restore step, but cross-cluster failover is not automatic without this orchestration.

## Multi-Cluster Failover Flow (CloudNativePG)

1. Run one CloudNativePG primary cluster and one or more replica clusters across Kubernetes clusters in the same region.
2. Keep replicas in continuous recovery via streaming replication, WAL archive, or both.
3. On primary cluster failure, control plane orchestrates promotion of a replica cluster (and fencing of the old primary if reachable).
4. Control plane switches the stable regional endpoint to the promoted primary.
5. Former primary is rejoined as replica or rebuilt from backup before returning to service.

## Future Note: Cross-Cluster Failover Complexity

- Manual DR promotion is feasible early and should be the first milestone.
- Fully automatic cross-cluster failover is a distributed-systems feature, not a single CloudNativePG toggle.
- Production-safe automation requires: reliable failure detection, old-primary fencing, deterministic promotion policy, endpoint cutover, and a clean rejoin/rebuild path.
- Recommended rollout: runbook-driven manual failover first, then guarded automation, then full automation after repeated failure-injection drills.

## Lifecycle States

`pending → provisioning → restoring → healthy → scaling → draining → failed → deleted`

## Cluster Drain Behaviour

- Replica: recreated on another cluster.
- Primary: failed over to a replica before drain completes.
- PVC: discarded.
- Data: recovered via replication or S3 restore.
- No manual migration required.

## Networking & Connection

- Users connect using a standard PostgreSQL connection string.
- **Public database**: `postgresql://user:password@<id>.eu-west.platform.dev:5432/dbname` — traffic enters via Cilium Gateway TCPRoute.
- **Private database**: `postgresql://user:password@<id>.internal:5432/dbname` — traffic stays inside the Cilium mesh, resolved via internal ClusterIP Service only.
- The platform provisions credentials via CloudNativePG and hands the user the connection string. Standard PostgreSQL auth (username/password + SSL) handles the rest.
- No intermediate auth proxy.

## Control Plane Responsibilities

- Owns: database metadata, scheduling decisions, credential issuance, backup policies, lifecycle transitions.
- Clusters own: execution, reconciliation, health reporting.

## Cluster Agent Responsibilities

- Creates CloudNativePG cluster resource.
- Creates PVC.
- Configures backup to S3.
- Sets up streaming replication.
- Reports placement health to control plane.

---

# Postgres Branching (Optional)

## What a Branch Is

- An independent database created from a point in the WAL history of a parent database.
- Implemented via backup restore + WAL replay to a target LSN + PostgreSQL timeline promotion.
- No full data copy. History before the branch point is shared via object storage.

## Resource Model

- `DatabaseBranch` — metadata resource with: id, database_id, name, parent_branch, recovery_lsn, workspace_id, region_id, status.
- One branch = one independent CloudNativePG cluster.
- Storage layout: `region/db_<id>/branches/<branch-name>/` — each branch has its own archive prefix.

## Branch Creation Flow

1. Control plane records current WAL LSN on the parent database.
2. Control plane creates `DatabaseBranch` record.
3. Cluster agent creates a new CloudNativePG cluster using recovery bootstrap targeting that LSN.
4. PostgreSQL replays WAL to LSN, promotes, starts new timeline. Branch is now independent.

## Timeline Isolation

- WAL history before branch point: shared with parent (via object storage).
- WAL after branch point: fully independent per branch.
- Compute: independent.
- No risk of corrupting parent or other branches.

## Networking

- Each branch gets a stable regional endpoint.
- Same routing model as a regular database.

## Lifecycle

- **Suspend** — branch compute removed, storage retained.
- **Resume** — cluster recreated from branch archive.
- **Delete** — cluster removed and archive prefix deleted.

## Guarantees

- Branch is isolated. Main database is never affected.
- Branch survives cluster loss (recovered from object storage).
- Branching does not duplicate full datasets.
- No instant startup guarantee on first creation (WAL replay takes time).
