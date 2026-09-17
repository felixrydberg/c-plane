# C-Plane

C-Plane is the platform control plane and regional S3-compatible storage
gateway.

## Getting started

For the first start, run the installer for the environment you want. It
creates the local configuration and secrets, initializes OpenBao, runs the
migrations, and prompts for the first S3 provider:

```bash
bash build.sh dev
# The script asks whether ingress should use a public domain.
# Answer "no" for local development without subdomains.

bash build.sh prod
# Answer "yes", then enter the public domain when prompted.
```

After the first start, run the environment you want:

```bash
# Development
docker compose -f docker-compose.dev.yml watch

# Production
docker compose -f docker-compose.prod.yml up
```

## Local metrics observability

The optional `docker-compose.observability.yml` overlay runs the
`grafana/otel-lgtm:0.33.0` development stack as one container. It scrapes the
API, UI, and ingress management metrics endpoints every 15 seconds and keeps
Grafana data in a Compose-managed volume. This is for local evaluation
only; it is not a production observability deployment. Logs, traces, alerts,
and public Grafana exposure are intentionally not configured.

Attach it to a running development or production Compose project:

```bash
# Development
docker compose -f docker-compose.dev.yml -f docker-compose.observability.yml up -d observability

# Production
docker compose -f docker-compose.prod.yml -f docker-compose.observability.yml up -d observability
```

Open the provisioned C-Plane dashboard at <http://127.0.0.1:3002> (Grafana's
default credentials are `admin` / `admin`). The application `/metrics`
endpoints remain internal to the Compose network; only Grafana is published.

## Frontend tooling

The UI uses Deno 2.9.3. For direct UI work, run:

```bash
deno install
deno task dev:ui
```
