# Cluster Agent — Refined Responsibilities

## Bootstrap (runs once when a cluster joins)

- Fetches the operator manifest from the control plane. The control plane is the source of truth for which operators are required and at which version.
- Installs each operator in the manifest before any workloads are scheduled.
- Reports cluster as ready to control plane only after all required operators are healthy.

## Operator Lifecycle

- The control plane maintains a versioned operator manifest: a list of operators, their versions, and whether they are required or optional.
- Required operators ship with the platform (Cilium, CloudNativePG, Kata Containers). Optional operators are enabled per cluster or per region (e.g. MongoDB, ClickHouse).
- Operator versions are pinned to the control plane version. When the control plane upgrades, it can push updated operator versions to clusters.
- On each reconciliation cycle the agent compares the installed operator versions against the manifest and upgrades or installs as needed.
- New operators can be added to the manifest by the control plane without changes to the agent itself, as long as the agent knows how to install the operator type (Helm chart, OLM, raw manifests).

## Render (runs in Rust CPlane package)

- The render pipeline runs in `packages/c-plane` and is the source of truth for how desired state is compiled for each cluster.
- Detailed render definitions are documented in `docs/cluster/render.md`.
- The agent consumes rendered desired state and applies it; it does not own render policy.

## Ongoing Reconciliation

### Heartbeat
- Reports cluster health, operator health, and desired-state apply status to control plane on each heartbeat.
- Periodically checks configured `cluster_ingress_endpoints` and reports health status updates.
- Updates `last_seen_at` for agent heartbeat.

## What the Cluster Agent Does Not Do

- Does not assign service identities (control plane owns this).
- Does not decide placement across clusters (scheduler owns this).
- Does not create ingress endpoint inventory (control plane API/integrations own this).
- Operators never intervene per deployment. All reconciliation is driven by control plane instructions.
  - The control plane sends a desired state to the cluster agent (what deployments should exist and what their config is).
  - The cluster agent compares that desired state against what currently exists in Kubernetes and reconciles the difference.
  - If a deployment is added, the agent creates the resources. If it is removed, the agent deletes them. If config changes, the agent updates them.
  - An operator cannot and should not manually create, edit, or delete deployment resources inside the cluster. Any manual change would be overwritten on the next reconciliation cycle.
  - The only operator actions that are valid are cluster-level registration and bootstrap. Everything below that is automated.
