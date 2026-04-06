# Control-Plane ⇄ Cluster Agent — Implementation Specification

This document defines the **desired system specification** for connecting Kubernetes clusters to the platform control plane using a secure agent-based architecture.

The purpose is to establish a consistent, secure, and region-aware mechanism for cluster registration, authentication, and reconciliation aligned with the platform’s multi-region architecture and data-scoped security model.

This is a **behavioral and architectural specification**, not an implementation guide.

---

## Goals

* Clusters join the platform as authenticated infrastructure identities
* Control plane remains the single source of truth
* Clusters operate as replaceable execution environments
* No Kubernetes credentials leave clusters
* Communication follows a zero-trust model
* Architecture supports multi-cluster regions and automatic recovery
* Works identically for self-hosted and cloud deployments

---

## Architectural Principle

The system separates authority from execution:

| Component     | Responsibility                       |
| ------------- | ------------------------------------ |
| Control Plane | Desired state, scheduling, authority |
| Cluster Agent | Reconciliation and reporting         |
| Kubernetes    | Workload execution                   |
| Database      | Source of truth                      |

Clusters never become authoritative components.

They execute instructions derived from control-plane state.

This aligns with the region and scheduling model where deployments target regions rather than clusters. 

---

## Agent Package

A new workspace package MUST be introduced:

```
packages/c-plane-agent
```

Requirements:

* Written in Go
* Integrated into the existing Go workspace/module structure
* Built as an independent binary
* Deployable inside Kubernetes clusters
* Shares workspace tooling and dependency standards

The agent represents the platform inside a cluster.

---

## Cluster Identity Model

Clusters are treated as first-class identities within the control plane.

Each cluster possesses:

* globally unique identifier
* region association
* lifecycle state
* capability metadata
* cryptographic identity

Cluster identity is infrastructure-scoped and completely separate from user or workspace identity domains defined in the data-scoped permission system. 

Clusters MUST NOT access tenant or identity databases directly.

---

## Registration Model

Cluster onboarding follows a controlled bootstrap flow:

1. Control plane creates a cluster record.
2. A short-lived join credential is generated.
3. The agent starts within the cluster using this credential.
4. The agent exchanges the credential for permanent identity.
5. Bootstrap credentials are revoked immediately.

Registration establishes trust but does not grant scheduling eligibility until health reporting begins.

---

## Authentication Model

### Bootstrap Authentication

* Uses short-lived join credentials.
* Single-use.
* Time-limited.
* Exists only for initial identity establishment.

### Runtime Authentication

After registration, all communication uses:

* mutual authentication
* rotating cryptographic identity
* outbound connections initiated by the agent

Authentication represents **cluster identity**, not user identity.

---

## Trust Boundary

The platform enforces strict separation:

```
Users → Control Plane → Database
Clusters → Control Plane only
```

Clusters never communicate with tenant storage systems.

All authorization decisions remain centralized, preserving database-enforced isolation guarantees. 

---

## Communication Model

The agent maintains a persistent outbound connection to the control plane.

Characteristics:

* long-lived bidirectional stream
* control plane publishes desired state
* agent publishes observed state
* no inbound connectivity required for clusters

This enables operation behind NAT and private networks.

---

## Agent Responsibilities

The agent acts as a reconciliation component.

It is responsible for:

* receiving desired workload placements
* installing and reconciling required platform operators (for example, Cilium and Kata Containers)
* applying platform state to Kubernetes
* reporting cluster health
* reporting capacity and availability
* maintaining heartbeat signals
* reflecting execution status back to the control plane

The agent does not make scheduling decisions.

---

## Ingress Metadata Reporting

To support region ingress routing, the agent must publish normalized ingress metadata to the control plane.

Population flow:

1. Agent watches Kubernetes Service state for cluster ingress exposure.
2. Agent observes cluster health and ingress readiness.
3. Agent reports observed values to the control plane.
4. Control plane persists routing metadata and serves it to ingress.

Required reported fields:

* `ingress_endpoint`: derived from cluster ingress Service VIP or external address.
* `ingress_enabled`: whether cluster endpoint is currently eligible for new ingress traffic.
* `ingress_weight`: balancing weight used by ingress selection policy.

Cilium remains the networking primitive provider (service reachability, VIP advertisement, policy enforcement).
The control plane remains the routing source of truth consumed by ingress.

---

## Control Plane Responsibilities

The control plane:

* stores cluster registry state
* validates cluster identity
* assigns workloads
* defines required operator policy (including approved operators and versions) per region or cluster class
* evaluates cluster health
* performs recovery reconciliation
* determines placement across clusters

Failure recovery follows the regional reconciliation model where lost placements are recreated on healthy clusters. 

---

## Cluster Lifecycle States

Clusters progress through managed lifecycle stages:

```
pending
→ bootstrapping
→ healthy
→ draining
→ offline
→ removed
```

Lifecycle transitions are controlled exclusively by the control plane.

---

## Authorization Scope

Cluster permissions are constrained by:

* region membership
* assigned capabilities
* operational role

Clusters may:

* receive workload instructions
* install and update control-plane-approved cluster operators
* report state
* reconcile resources

Clusters may not:

* access other regions
* modify scheduling decisions
* install non-approved operators outside assigned policy
* access workspace data
* query platform databases

---

## Networking Assumptions

Within a region:

* clusters participate in shared networking through the existing mesh layer
* networking provides connectivity only
* scheduling and recovery remain control-plane responsibilities

Networking MUST NOT become an authorization mechanism.

---

## Failure Detection

Cluster health is determined through continuous reporting.

Loss of connectivity implies cluster unavailability.

Control plane reconciliation restores missing workloads automatically, ensuring regional availability guarantees.

---

## Deployment Model

The agent is deployed into Kubernetes using platform-provided installation tooling.

Installation establishes identity but does not embed long-term credentials into manifests.

After identity establishment, operator reconciliation is driven by control-plane policy so required cluster capabilities (such as networking via Cilium and sandboxed execution via Kata Containers) converge automatically.

Clusters remain safely replaceable infrastructure units.

---

## Security Properties

The system must guarantee:

* zero Kubernetes credential export
* revocable cluster identity
* automatic credential rotation
* least-privilege infrastructure access
* policy-bound operator lifecycle management for required cluster operators (including Cilium and Kata Containers)
* isolation from tenant data systems
* compatibility with transaction-scoped authorization architecture

---

## Expected Result

The platform gains:

* cloud-style cluster onboarding
* secure multi-cluster expansion
* region-level abstraction for users
* automatic recovery from cluster failure
* infrastructure identity separate from user identity
* consistent behavior across OSS and hosted environments

Clusters behave as authenticated executors of control-plane intent rather than independent control surfaces.
