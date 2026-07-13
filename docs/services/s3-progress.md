# S3 implementation progress

Updated 2026-07-12.

## Current state

- `bucket` now has a globally unique logical name and a `provisioning | ready | deleting | failed` lifecycle. The Drizzle migration is `0003_wonderful_black_queen.sql`.
- `packages/storage` is an unfinished regional service crate. It currently contains:
  - pinned `s3s` / `s3s-aws` and AWS SDK dependencies;
  - environment configuration;
  - canonical immutable namespace-node encoding and hashing;
  - path-copy insert/delete/get/list logic with snapshot roots;
  - signed continuation tokens;
  - SSE-C key-triplet and MD5 validation.
- The storage crate is wired into both Compose files as the `storage` service on port 8081.
- Branch-scoped S3 access tokens can be created, listed, and revoked from the Storage page.
- Token metadata is stored in Postgres. The recoverable SigV4 secret is stored only in OpenBao and is returned to the user once.
- The placeholder S3 endpoint resolves credentials through the control plane, verifies SigV4/SigV2 through `s3s`, and implements `ListBuckets` as an empty successful response. Other S3 operations return `NotImplemented`.
- The Rust workspace compiles, the storage unit tests pass, and the storage Vue page passes isolated ESLint. The repository-wide Nuxt typecheck still fails on pre-existing errors and a missing Vue tooling module.

## Testing the placeholder

After applying the Drizzle migrations and starting Postgres, OpenBao, control plane, API, UI, and storage:

1. Select a project and branch on the Storage page.
2. Create an S3 access token and save the one-time secret.
3. Run:

```powershell
$env:AWS_ACCESS_KEY_ID = "<access key ID>"
$env:AWS_SECRET_ACCESS_KEY = "<secret access key>"
aws s3api list-buckets --endpoint-url http://localhost:8081 --region local
```

The expected result is an empty bucket list. A revoked token must return an authentication error.

## Deferred after credential flow works

- FoundationDB-backed nodes, roots, generations, uploads, reference counts, and release jobs.
- Provider bucket provisioning and OpenBao bucket SSE-C keys.
- Object, listing, copy, delete, and multipart implementations.
- Project-branch root cloning and historical revision pins.
- Storage API/worker deployment wiring and the compatibility suite.
