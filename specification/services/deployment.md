
---

# 🚀 Deployment Specification (Platform)

## 🎯 Purpose

A **Deployment** describes:

> *What workload should run*, not *how infrastructure works*.

It defines:

* workload runtime
* scaling behavior
* storage usage
* exposure intent
* dependencies

It does **NOT** define:

❌ cluster placement
❌ networking topology
❌ Kubernetes objects
❌ tenant isolation rules

Those belong to platform subsystems.

---

# 1. Core Principle

A deployment exists **inside a tenant (organization)**.

Networking is inherited automatically:

```text
Deployment
    ↓
Organization
    ↓
Tenant Virtual Network (TVN)
    ↓
Region Network
```

Deployment specs only reference networking — they never configure it.

---

# 2. Resource Identity

Each deployment is uniquely identified by:

```yaml
deployment:
  id: dep_xxxx
  organization_id: ws_xxxx
  region: eu-west
```

Ownership determines:

* permissions
* network membership
* storage scope
* service discovery domain

---

# 3. High-Level Structure

```yaml
apiVersion: platform.dev/v1
kind: Deployment

metadata:
  name: api
  organization: ws-123

spec:
  runtime: {}
  scale: {}
  storage: {}
  network: {}
  health: {}
```

---

# 4. Runtime Specification

Defines container execution.

```yaml
runtime:
  image: ghcr.io/org/api:latest
  command: ["./server"]
  ports:
    - name: http
      port: 8080

  resources:
    cpu: 500m
    memory: 512Mi

  env:
    - name: ENV
      value: production
```

Control plane translates this into Kubernetes workloads.

---

# 5. Scaling

Platform-level intent only.

```yaml
scale:
  min: 1
  max: 10
  target_cpu: 70
```

Scheduler decides:

* cluster placement
* balancing across region
* rescheduling

Clusters remain disposable compute .

---

# 6. Storage

Aligned with platform storage model.

```yaml
storage:
  ephemeral:
    size: 5Gi

  persistent:
    - name: data
      class: working
      size: 20Gi

  object_access: true
```

Meaning:

* ephemeral → node disk
* persistent → recoverable PVC
* durability → S3 object storage (region layer) 

---

# 7. Networking (Pointer Only)

⚠️ **This is intentionally minimal.**

Networking belongs to the **organization virtual network**.

```yaml
network:
  service: api
  expose:
    - port: http
```

That’s it.

---

## What this means

The platform automatically:

* attaches deployment to organization network
* assigns service identity
* creates DNS records
* generates Cilium policies
* enforces tenant isolation

As defined in tenant networking .

---

### Resulting Service Identity

Automatically becomes:

```
api.ws-123.internal
```

No networking YAML required.

---

# 8. Service Dependencies (Optional Hint)

Deployment may declare *intent*, not policy.

```yaml
network:
  service: api
  allow:
    - redis
    - worker
```

Control plane converts this into:

* internal allow rules
* service discovery entries

Users never write network policies directly.

---

# 9. Health & Lifecycle

```yaml
health:
  readiness:
    http:
      path: /ready
      port: http

  liveness:
    http:
      path: /health
      port: http
```

Used by:

* scheduler
* traffic routing
* rescheduling decisions

---

# 10. What Deployment Spec MUST NOT Contain

To preserve platform architecture:

| Forbidden             | Reason                     |
| --------------------- | -------------------------- |
| namespaces            | platform-owned             |
| pod networking        | tenant network abstraction |
| node selectors        | clusters disposable        |
| ingress configs       | regional routing layer     |
| provider storage info | abstracted via platform    |
| cluster IDs           | hidden infrastructure      |

---

# 11. Control Plane Responsibilities

When a deployment is created:

1. Validate organization ownership
2. Attach deployment → tenant network
3. Allocate service identity
4. Generate desired state
5. Send to selected cluster agents
6. Agents reconcile Kubernetes objects

---

# 12. Mental Model (Important)

User thinks:

> “Deploy an app in my project.”

Platform actually does:

```text
Deployment Spec
     ↓
Organization Network
     ↓
Regional Scheduler
     ↓
Cluster Agent
     ↓
Kubernetes Resources
```

---

# 13. Minimal Example

```yaml
apiVersion: platform.dev/v1
kind: Deployment

metadata:
  name: api
  organization: ws-123

spec:
  runtime:
    image: ghcr.io/acme/api:v1
    ports:
      - name: http
        port: 8080

  scale:
    min: 2
    max: 5

  network:
    service: api
    expose:
      - http
```

User never touches networking beyond naming intent.

---

# ✅ One-Sentence Definition

> A deployment specifies workload intent while inheriting connectivity automatically from the organization’s tenant virtual network.

---
