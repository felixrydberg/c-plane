
---

# 🌐 Tenant Networking Specification

## 🎯 Goal

Pods belonging to the **same tenant (workspace)** must be able to:

✅ communicate privately
✅ discover services automatically
✅ work across clusters inside a region
✅ survive rescheduling
✅ remain isolated from other tenants

Without exposing Kubernetes complexity.

---

# 1. Core Mental Model

Your platform networking model is:

```
Region Network
    ↓
Tenant Virtual Network
    ↓
Services
    ↓
Pods (ephemeral)
```

A tenant does **not** get a Kubernetes namespace.

They get a **virtual network overlay** spanning clusters.

---

# 2. Networking Layers

| Layer                  | Responsibility             |
| ---------------------- | -------------------------- |
| Cilium Cluster Mesh    | cross-cluster connectivity |
| Platform Network Layer | tenant isolation           |
| Service Discovery      | naming & routing           |
| Kubernetes             | packet delivery only       |

Cilium provides connectivity — **you provide tenancy**.

---

# 3. Region Networking Foundation

Inside a region:

* all clusters participate in one **Cilium Cluster Mesh**
* pods receive routable IPs
* cross-cluster traffic works automatically

Result:

> Every pod in a region can technically reach every other pod.

Your job is to **restrict this safely**.

(Connectivity first, isolation second.)

---

# 4. Tenant Virtual Network (TVN)

## Definition

Each workspace receives a logical:

```
Tenant Virtual Network (TVN)
```

Conceptually similar to:

* AWS VPC
* Fly.io org network
* Kubernetes namespace + policies (but platform-owned)

---

## Identity

Each pod receives labels:

```yaml
platform.workspace_id=ws_123
platform.deployment_id=dep_abc
```

These labels are authoritative.

Agents inject them automatically.

Users cannot modify them.

---

# 5. Isolation Mechanism (CRITICAL)

Isolation is enforced using **Cilium Network Policies**.

Default rule:

```
DENY ALL cross-workspace traffic
```

---

## Default Policy

Allow traffic only when:

```
source.workspace_id == destination.workspace_id
```

Conceptually:

```text
workspace A → workspace A ✅
workspace A → workspace B ❌
```

This becomes your primary tenant isolation boundary.

---

# 6. Service Identity (NOT Pod Identity)

Pods are ephemeral.

Networking must target **services**, not pods.

---

## Platform Service Name

Each deployment gets:

```
<service>.<workspace>.svc.region
```

Example:

```
api.ws-123.svc.eu-west.internal
```

Stable regardless of placement.

---

### Resolution Flow

```
Pod
 ↓ DNS
Regional Service Name
 ↓
Cilium global service
 ↓
Healthy endpoints across clusters
```

Cilium automatically load-balances.

---

# 7. Cross-Cluster Communication

Because of Cluster Mesh:

```
Pod A (Cluster 1)
        ↓
Pod B (Cluster 2)
```

Works transparently.

No gateways required.

This matches your regional abstraction .

---

# 8. Service Discovery

You run a **platform DNS layer**.

### DNS Structure

```
.internal.region
```

Examples:

| Type       | Example                |
| ---------- | ---------------------- |
| Service    | api.ws-123.internal    |
| Database   | db.ws-123.internal     |
| Job worker | worker.ws-123.internal |

DNS resolves to regional service VIPs.

---

# 9. Zero-Trust Defaults

Inside a tenant network:

### Default behavior

```
deny all
```

Then allow explicitly:

| Source              | Destination |
| ------------------- | ----------- |
| same deployment     | allowed     |
| declared dependency | allowed     |
| external ingress    | controlled  |

---

## Service-to-Service Authorization (Future)

Later you can add:

* workload identity
* mTLS between services
* service auth tokens

But **network isolation comes first**.

---

# 10. How Deployments Declare Connectivity

Deployment spec includes:

```yaml
network:
  expose:
    - http
  allow:
    - service: api
    - service: cache
```

Control plane converts this into:

* Cilium policies
* service entries
* DNS records

Users never write network policies.

---

# 11. Networking Lifecycle

## Deployment Created

Control plane:

1. assigns workspace network
2. registers service name
3. generates policies
4. sends config to agents

---

## Agent Applies

Agent creates:

* Kubernetes Service
* Cilium policy
* labels on pods

---

## Runtime

Traffic allowed only if policies match.

---

# 12. Failure & Rescheduling

When pods move clusters:

✅ IP changes
✅ service identity unchanged
✅ DNS unchanged
✅ policies unchanged

Networking remains stable.

---

# 13. Security Boundaries

| Boundary            | Enforcement     |
| ------------------- | --------------- |
| Workspace isolation | Cilium policy   |
| Service exposure    | platform config |
| External access     | ingress layer   |
| Data isolation      | Postgres RLS    |

Notice the symmetry:

```
Database → RLS isolation
Network → policy isolation
```

Same philosophy.

---

# 14. Internal vs External Traffic

## Internal (tenant network)

```
service.ws.internal
```

Private only.

---

## External (internet)

Handled separately via:

* regional ingress gateway
* TLS termination
* routing layer

(Not part of tenant networking itself.)

---

# 15. Minimal Example

User deploys:

```
api
worker
redis
```

Platform automatically enables:

```
api ↔ redis
worker ↔ redis
worker ↔ api
```

But blocks:

```
other_workspace → redis ❌
```

No YAML required.

---

# 16. Developer Mental Model

User thinks:

> “My services in a project can talk privately.”

Reality:

```
workspace labels
→ platform policies
→ Cilium enforcement
→ cross-cluster routing
```

---

# ✅ Final Architecture Diagram

```
                REGION NETWORK (Cilium Mesh)
 ─────────────────────────────────────────────

      Workspace A Network        Workspace B Network
      ┌──────────────────┐       ┌──────────────────┐
      │ api ↔ worker ↔ db │       │ api ↔ db         │
      └─────────┬────────┘       └─────────┬────────┘
                │                          │
          policies isolate tenants completely
```

---

## ⭐ One-Sentence Definition

> Tenant networking is implemented as a region-wide virtual network enforced by Cilium policies that allow communication only between workloads sharing the same workspace identity.

---
