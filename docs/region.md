# Region — Refined Decisions

## What a Region Is

- A region is the deployment target exposed to users. Users pick a region, never a cluster.
- A region maps to one Cilium Cluster Mesh providing a flat internal network.
- Regions are persistent platform resources and never change from the user's perspective, even as clusters within them change.

## Requirements

- Regions are stored in the control-plane database.
- Organization access to regions is controlled through capabilities.
- Deployments are bound to a region, never to a specific cluster.
- Regions remain independent of organizations.

## Clusters Within a Region

- A region contains one or more clusters.
- Clusters are not user-visible. They are infrastructure units.
- Each cluster belongs to exactly one region.
- Clusters can be added or removed without affecting deployments.
- All clusters in a region share service discovery via Cilium Cluster Mesh.

## High Availability

- HA is achieved by running multiple clusters per region.
- Replicas of a deployment can be distributed across clusters (active-active).
- If one cluster fails, the control plane reschedules workloads on remaining healthy clusters.
- No user action is required for recovery.

## Scheduling

- Control plane selects a healthy cluster within the target region when a deployment is created.
- Cluster selection is based on health and capacity.
- The resulting placement record links a deployment to a specific cluster.
- Placements are internal records. They can be recreated without changing the deployment.

## Failure Recovery

1. Cluster goes offline.
2. Placements on that cluster are marked lost.
3. Control plane detects missing replicas.
4. Replacement placements are scheduled on healthy clusters in the same region.
5. Networking updates once new endpoints are live.

## Responsibility Boundary

- Regions: user-facing deployment target, durability boundary.
- Clusters: replaceable execution units.
- Cilium Mesh: connectivity and service discovery within the region.
- Render pipeline: source of truth for region mesh bootstrap and peer topology output.
- Control plane: scheduling, reconciliation, recovery.
