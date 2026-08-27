# Secrets library

Postgres is the source of truth for encrypted secret records. OpenBao Transit
protects their ciphertext and never stores the application secret payloads.
API and control plane use the reusable library in the
`packages/lib` Rust crate; there is no secrets service or secret-proxy process.

```rust
let client = secrets::Client::from_env()?;
let ciphertext = secrets::encrypt(&client, secrets::PLATFORM_KEY, plaintext).await?;
let plaintext = secrets::decrypt(&client, secrets::PLATFORM_KEY, &ciphertext).await?;
```

The crate contains no domain authorization. OpenBao AppRole policies determine
which paths each process may access:

- API may encrypt and decrypt platform and tenant secrets.
- Control plane may encrypt and decrypt platform secrets.

`OPENBAO_TOKEN` is an explicit development override. Without it, the library
logs in with `OPENBAO_ROLE_ID` and `OPENBAO_SECRET_ID`, caches the returned
token, and retries one login when OpenBao rejects an expired token.

## Transit keys

| Scope | Transit key |
| --- | --- |
| Platform | `platform` |
| Tenant | `tenant-{organization_id_without_hyphens}` |

OpenBao KV is not enabled. Products that have not migrated credentials to
Transit-backed Postgres rows fail closed until their migration lands.

Secret rows carry platform or tenant scope. Tenant rows identify their
organization and are protected by RLS. Credentials retain only their public
access-key identifier and reference a Secret for sensitive material.

Installation enables Transit, creates the platform key, writes API and
control-plane policies, and passes no OpenBao root token to a
long-running application container.

The versioned policy definitions live in `packages/openbao/policies`. The
installer applies those HCL files; only generated role and secret IDs remain
dynamic installation state.
