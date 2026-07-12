# Rust control plane

This Dioxus full-stack application is the private infrastructure control plane.
It manages organizations, API keys, regions, clusters, S3 providers, and audit
logs. Infrastructure mutations are written to `infrastructure_audit_log`.
S3 credentials live only in OpenBao and an in-memory cache; the database stores
provider metadata.

## Development

Install `dioxus-cli` 0.7.9, then run from this directory:

```powershell
$env:ADMIN_DATABASE_URL = "postgresql://cplane_admin:password@localhost:5432/cplane"
$env:OPENBAO_ADDR = "http://127.0.0.1:8200"
$env:OPENBAO_TOKEN = "dev-only-token"
$env:CPLANE_SERVICE_TOKEN = "a-long-random-service-token"
dx serve --web --addr 127.0.0.1 --port 3001 --open false --fullstack true
```

Docker Compose publishes the UI only on `127.0.0.1:3001`. The internal S3
credential endpoint additionally requires `CPLANE_SERVICE_TOKEN` and must not
be exposed through a public reverse proxy.
