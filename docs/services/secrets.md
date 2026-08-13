# Secrets library

OpenBao is the source of truth for secret values. API and control plane use the
the secret service exposed by the reusable `packages/lib` Rust crate; there is
no secrets service or secret-proxy process.

```rust
let secrets = Secrets::from_env()?;

let value = secrets.get::<T>("path").await?;
secrets.set("path", &value).await?;
secrets.delete("path").await?;
```

The crate contains no domain authorization. OpenBao AppRole policies determine
which paths each process may access:

- API may read provider credentials and manage access-key records, bucket keys,
  and organization registry secrets.
- Control plane may manage provider credentials only.

`OPENBAO_TOKEN` is an explicit development override. Without it, the library
logs in with `OPENBAO_ROLE_ID` and `OPENBAO_SECRET_ID`, caches the returned
token, and retries one login when OpenBao rejects an expired token.

## Paths

| Data | KV v2 path |
| --- | --- |
| S3 provider credential | `platform/s3/providers/{provider_id}` |
| S3 access-key record | `platform/s3/access-keys/{access_key_id}` |
| Bucket SSE-C key | `storage/sse-c/{bucket_id}` |
| External registry token | `organizations/{organization_id}/registries/{registry_id}` |

Postgres remains authoritative for active state, ownership, and permissions.
An access-key record contains only the IDs needed for API to establish the
database scope before checking current state.

Installation enables AppRole, writes both policies, creates role and secret
IDs, provisions registry access-key records and the registry bucket key, and
passes no OpenBao root token to a long-running application container.

The versioned policy definitions live in `packages/openbao/policies`. The
installer applies those HCL files; only generated role and secret IDs remain
dynamic installation state.
