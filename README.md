# C-Plane

C-Plane is the platform control plane and regional S3-compatible storage
gateway.

## Getting started

For the first start, run the installer for the environment you want. It
creates the local configuration and secrets, initializes OpenBao, runs the
migrations, and prompts for the first S3 provider:

```bash
./install.sh dev
# or
./install.sh prod
```

After the first start, run the environment you want:

```bash
# Development
docker compose -f docker-compose.dev.yml watch

# Production
docker compose -f docker-compose.prod.yml up
```

## Frontend tooling

The UI uses Deno 2.9.3. For direct UI work, run:

```bash
deno install
deno task dev:ui
```
