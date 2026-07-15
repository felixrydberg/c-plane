# C-Plane

C-Plane is the platform control plane and regional S3-compatible storage
gateway.

## Local development

Copy `.env.example` to `.env` and set the application passwords and tokens.
OpenBao uses a persistent single-node Raft volume, so it must be initialized
once before starting the full development stack.

```powershell
docker compose -f docker-compose.dev.yml up -d openbao
docker compose -f docker-compose.dev.yml exec -e BAO_ADDR=http://127.0.0.1:8200 openbao bao operator init -key-shares=1 -key-threshold=1
```

Copy the command's `Initial Root Token` and `Unseal Key 1` into `.env`:

```dotenv
OPENBAO_ROOT_TOKEN=...
OPENBAO_UNSEAL_KEY=...
```

Do not commit these values. Start the development stack normally afterwards:

```powershell
docker compose -f docker-compose.dev.yml up --build
```

The `openbao_data` Docker volume retains OpenBao data across container
restarts. On each startup, the `openbao-init` service uses
`OPENBAO_UNSEAL_KEY` to unseal OpenBao and ensures the `cplane` KV mount is
available.
