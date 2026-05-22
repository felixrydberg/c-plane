# Postgres — Refined Decisions

## Deployment Modes

Postgres databases run in one of two modes: **Stateful** or **Serverless**.

### Stateful (Xata Operator)

A dedicated Postgres cluster per database with persistent compute, managed by the [Xata operator](https://github.com/xataio/xata-operator).

- Always-on primary instance with optional replicas in the same Kubernetes cluster.
- Predictable performance. No cold starts. Connection pools remain active.
- Best for production workloads, high-throughput applications, and databases that must respond immediately.
- Autoscaling: CPU and memory can change within operator-defined limits. Vertical scaling may trigger a brief restart but does not drop connections to replicas.
- Cost: compute runs continuously regardless of traffic. Suitable for databases that are never idle.

### Serverless (Neon Operator)

A Postgres instance that scales to zero when idle, managed by a custom [Neon](https://neon.tech)-based operator.

- Compute is suspended after an idle period (no active connections). On the next connection, the operator resumes the instance using Neon's branch-and-restore compute model. Cold start is bounded by page replay from storage.
- Best for development branches, preview environments, staging databases, and workloads with intermittent traffic.
- Autoscaling: compute scales up during load and scales to zero when idle. Scaling behaviour is configured per database.
- Cost: compute cost is proportional to active time. Storage cost is unchanged.

### Choosing Between Them

| Concern | Stateful | Serverless |
|---------|----------|------------|
| Cold start | None | Page replay on resume |
| Compute cost | Always-on | Proportional to active time |
| Idle behaviour | Running, minimal cost | Suspended, zero compute cost |
| Connection persistence | Connections stay alive | Connections drop on suspend |
| Use case | Production, APIs, high traffic | Branches, staging, preview, batch |

Each mode uses its own operator with different architecture. The following sections note which mode(s) they apply to.

---

## What a Postgres Database Is

*Applies to: both modes*

- A region-scoped managed database service.
- Users deploy databases to regions, never to clusters.
- That Kubernetes cluster is disposable. The database is not.

---

## Resource Model

*Applies to: both modes*

- `Database` — logical database resource owned by a workspace and a region.
- `DatabasePlacement` — internal record linking a database to the Kubernetes cluster currently hosting it. Never user-visible.
- `DatabaseStorage` — backup bucket, WAL prefix, and retention policy. Always region-scoped object storage.

---

## Durability Model

*Applies to: Stateful (Xata)*

- Durability comes from in-cluster replication + WAL shipping + continuous S3 backups. Not from the PVC.
- PVCs are working storage only. They are discarded on cluster drain.
- If a cluster dies: new cluster restores from S3, replays WAL, resumes service.
- Minimum self-host requirement: Kubernetes + S3-compatible storage. No Ceph required.

*Applies to: Serverless (Neon)*

- Durability is managed by the Neon operator's page-level storage architecture with safekeepers.
- Compute nodes are ephemeral; all data is persisted to shared storage before suspension.
- On resume, the operator creates a new compute node that reads from the same storage layer.

---

## High Availability Modes (Future)

*Applies to: Stateful (Xata)*

- **Single primary (reactive)** — one primary instance, backups to S3. On failure, restore from backup and start a new primary. Simplest mode.
- **Single-cluster HA (preferred)** — one Xata-managed Postgres cluster with primary + replicas in the same Kubernetes cluster. Pod/node failures are handled by local failover; whole-cluster loss is handled by restore to a replacement cluster.

*Applies to: Serverless (Neon)*

- HA is provided by the Neon operator's compute-storage separation. Failed compute nodes are replaced on resume.

---

## Cluster Relocation Flow (Single-Cluster Deployment)

*Applies to: Stateful (Xata)*

1. Run one Postgres cluster per database on one Kubernetes cluster.
2. Keep multiple Postgres instances inside that cluster for local failover.
3. On full Kubernetes cluster failure or drain, control plane creates a replacement cluster on another Kubernetes cluster.
4. Replacement cluster restores from base backup, replays WAL from object storage, and becomes healthy.
5. Control plane switches the stable regional endpoint to the replacement cluster.

---

## Future Note: Recovery Automation Complexity

*Applies to: Stateful (Xata)*

- Manual DR promotion is feasible early and should be the first milestone.
- Fully automatic cluster-loss recovery is a distributed-systems feature.
- Production-safe automation requires: reliable failure detection, deterministic restore policy, endpoint cutover, and a clean rejoin/rebuild path.
- Recommended rollout: runbook-driven manual failover first, then guarded automation, then full automation after repeated failure-injection drills.

---

## Lifecycle States

*Applies to: Stateful (Xata)*

`pending → provisioning → restoring → healthy → scaling → draining → failed → deleted`

*Applies to: Serverless (Neon)*

`pending → provisioning → healthy → suspended → resuming → deleted`

---

## Cluster Drain Behaviour

*Applies to: Stateful (Xata)*

- Control plane creates a replacement Postgres cluster on another Kubernetes cluster.
- Replacement restores from S3 and replays WAL.
- PVC: discarded.
- Data: recovered via in-cluster replication (for pod/node faults) or S3 restore (for cluster relocation).
- Endpoint: switched to replacement cluster after health checks pass.
- No manual migration required.

*Applies to: Serverless (Neon)*

- Compute nodes are ephemeral and replaced on resume. No drain migration needed.
- Storage is always preserved in the shared storage layer.

---

## Networking & Connection

*Applies to: both modes*

- Users connect using a standard PostgreSQL connection string.
- **Public database**: `postgresql://user:password@<id>.eu-west.platform.dev:5432/dbname` — traffic enters via Cilium Gateway TCPRoute.
- **Private database**: `postgresql://user:password@<id>.internal:5432/dbname` — traffic stays inside the Cilium mesh, resolved via internal ClusterIP Service only.
- The platform provisions credentials and hands the user the connection string. Standard PostgreSQL auth (username/password + SSL) handles the rest.
- No intermediate auth proxy.

---

## Control Plane Responsibilities

*Applies to: both modes*

- Owns: database metadata, scheduling decisions, credential issuance, backup policies, lifecycle transitions.
- Clusters own: execution, reconciliation, health reporting.

---

## Cluster Agent Responsibilities

*Applies to: Stateful (Xata)*

- Creates Xata Postgres cluster resource.
- Creates PVC.
- Configures backup to S3.
- Configures streaming replication.
- Reports placement health to control plane.

*Applies to: Serverless (Neon)*

- Creates Neon compute node resource on resume.
- Configures connection to shared storage.
- Reports compute health to control plane.
