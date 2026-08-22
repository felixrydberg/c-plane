# C-Plane ingress

Pingora reverse proxy for C-Plane's public HTTP surface. It terminates no TLS and assumes no particular upstream tunnel or load balancer.

## Routes

| Authority | Path | Upstream |
| --- | --- | --- |
| Platform | `/api/**` | Rust API |
| Platform | `/ui-api/**` | Nuxt/Nitro |
| Platform | everything else | Nuxt UI |
| API | `/api/**`, `/health` | Rust API |
| Storage, including configured bucket subdomains | exact path | Storage API |
| Registry | `/v2/**` | Distribution Registry |

Storage and Registry paths are never rewritten. This preserves S3 signatures and the Registry protocol.

## Subdomains

Each service can use its own public authority while sharing one Pingora listener:

```env
INGRESS_LISTEN=0.0.0.0:8080
INGRESS_PLATFORM_HOSTS=app.example.com
INGRESS_API_HOSTS=api.example.com
INGRESS_STORAGE_HOSTS=storage.example.com
INGRESS_STORAGE_HOST_SUFFIXES=storage.example.com
INGRESS_REGISTRY_HOSTS=registry.example.com
```

This routes:

- `app.example.com/**` to Nuxt, except `/api/**` to Rust and `/ui-api/**` to Nitro
- `api.example.com/api/**` to Rust
- `storage.example.com/**` and `<bucket>.storage.example.com/**` to Storage
- `registry.example.com/v2/**` to the Registry

The platform host may also be an apex domain such as `example.com`. Host variables accept comma-separated aliases. DNS, TLS, and forwarding traffic to the listener are deployment concerns outside this package.

For local development, the defaults use ports because wildcard localhost subdomains are not portable:

| URL | Service |
| --- | --- |
| `http://localhost:3000` | Nuxt, Nitro, and same-origin `/api/**` |
| `http://localhost:8080` | Rust API |
| `http://localhost:8081` | Storage |
| `http://localhost:5000` | Registry |

## Rate controls

`INGRESS_RATE_MODE` is `off`, `observe` (default), or `enforce`. Each traffic class uses a compact `requests-per-second,requests-per-minute,in-flight` setting, for example:

```text
INGRESS_RATE_API_READ=30,600,64
```

Request rates are local per-client guardrails and return `429` in enforce mode. In-flight limits are local per-replica saturation guards and return `503`. Replicas share no state, so ingress remains horizontally scalable; durable account or tenant quotas belong in the services.

Client IP headers are ignored unless both `INGRESS_CLIENT_IP_HEADER` and `INGRESS_TRUSTED_PROXY_CIDRS` are set. The configured header must contain exactly one IP address.

See `config.rs` for the full environment-variable list and defaults.
