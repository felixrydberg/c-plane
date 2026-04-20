# Render Pipeline — Refined Decisions

## Purpose

- The render pipeline runs in `packages/c-plane`.
- It compiles control-plane intent into deterministic, per-cluster desired state.
- Rendering is control-plane-only logic. Agents execute rendered output and report status.

## Inputs

- Deployment intent: what workloads should exist, runtime config, and exposure mode.
- Connectivity intent: allowed service-to-service edges.
- Cluster context: provider, health, capacity, and registered ingress endpoints.
- Region context: region status, routing mode, and cluster membership.
- Region mesh context: mesh domain, cluster mesh identities, and eligible peer endpoints.
- Operator manifest pinned to control-plane version.
- Capability flags used for feature gating.

## Render Stages

1. Build a consistent snapshot of control-plane state.
2. Plan region mesh membership and per-cluster peer sets.
3. Build provider-agnostic desired models.
4. Apply provider-specific overlays using cluster `provider`.
5. Compile Kubernetes-facing resources.
6. Canonicalize output and compute generation/hash.
7. Persist rendered state and serve it to cluster agents.

## Region Mesh Rendering

- Each region maps to a single Cilium Cluster Mesh domain.
- Each cluster gets a stable mesh identity within its region.
- The render pipeline computes peer topology from healthy, mesh-eligible clusters in the same region.
- Rendered output includes per-cluster mesh bootstrap data: local identity, peer endpoints, and trust material references.
- Provider agnostic behavior is enforced by using one endpoint abstraction across all providers.
- Provider integrations only populate endpoint inventory and provider metadata; they do not change mesh semantics.
- On-prem clusters use the same contract by submitting static endpoints through control-plane API.
- Cluster add/remove, endpoint changes, or health transitions trigger re-render for affected region clusters.

## Rendered Resources

### Per-deployment resources

For each deployment, the render pipeline defines:
- `Deployment` — runs workload pods.
- `Service` — stable in-cluster endpoint for pods.
- `HTTPRoute` — only when public exposure is requested.

For TCP-based services (e.g. Postgres), the render pipeline defines `TCPRoute` instead of `HTTPRoute`:
- `TCPRoute` routes raw TCP traffic from a Gateway TCP listener to the Service.
- Public databases get a `TCPRoute`; private databases use ClusterIP only.

### Region mesh resources

The render pipeline defines region mesh configuration per cluster, including:
- Cluster mesh identity and mesh peer references.
- Mesh connectivity configuration derived from region peer topology.
- Any required policy and secret references used for mesh bootstrap.

## Output Contract

- Desired state is full-state and versioned.
- The payload is deterministic: identical inputs yield identical generation/hash.
- Render output includes operator, workload, network policy, and region mesh sections.
- Agents apply by diff-and-prune semantics and report applied generation plus health.
- Control plane uses heartbeat state for drift detection and targeted re-render triggers.
