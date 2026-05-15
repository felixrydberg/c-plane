# Projects

Projects are the primary application abstraction within the platform.

A project defines:

* containers,
* databases,
* networking,
* storage,
* environment configuration,
* and deployment topology.

Projects are declarative and revision-based. Every project change creates a new immutable revision that is rendered into desired runtime state by the control plane.

Projects are regional abstractions and are not bound to individual Kubernetes clusters.

---

# Project Structure

A project may contain:

* container services,
* PostgreSQL databases,
* Redis instances,
* object storage buckets,
* scheduled jobs,
* internal networking,
* public ingress configuration,
* secrets references.

Example:

```yaml
apiVersion: platform.dev/v1
kind: Project

spec:
  containers:
    - name: api
      image: ghcr.io/acme/api:latest

    - name: worker
      image: ghcr.io/acme/worker:latest

  databases:
    - name: app-db
      engine: postgres
```

---

# Revisions

Projects are immutable.

Every project update creates a new revision.

Revisions provide:

* rollback,
* deployment history,
* auditability,
* deterministic rendering,
* reproducible environments.

The control plane renders revisions into concrete desired state for cluster agents.

---

# Branches

Projects support isolated branches.

Branches behave similarly to Git branches and allow independent project environments to exist simultaneously.

Example:

```text
main
 ├── staging
 ├── feature/auth
 └── feature/payments
```

Each branch maintains:

* independent revisions,
* isolated containers,
* isolated databases,
* isolated networking,
* isolated runtime state.

Changes made to one branch do not affect other branches. They are not expected to be merged or long lived either. They are expected to be short lived and may be deleted when no longer needed.

---

# Rendering

Projects are rendered by the control plane into:

* Kubernetes workloads,
* networking resources,
* storage configuration,
* database topology,
* routing configuration.

Rendering is deterministic.

The same:

* project revision,
* platform version,
* and placement configuration

will always produce the same desired state output.
