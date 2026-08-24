# Postgres Backup Storage Isolation

## Decision

Postgres backups use one private physical bucket and one backing-provider
credential per region. The regional Storage API is the only component that
receives that credential.

Every database branch receives a derived Storage API credential and an
immutable backup prefix. The credential identifies the branch; it is not a
regional credential copied into each CloudNativePG cluster. Storage resolves
the branch's organization from control-plane state, enforces its prefix, and
injects that organization's regional SSE-C key when proxying requests to the
backing provider.

```text
CloudNativePG branch
  |  derived branch credential
  v
regional Storage API
  |-- authenticate branch ownership
  |-- enforce organization/database/branch prefix
  |-- inject organization-and-region SSE-C key
  v
regional physical bucket
  |  regional backing-provider credential
  v
S3-compatible provider
```

This keeps physical storage regional, encryption organizational, and request
authority branch-scoped.

## Resource Boundaries

| Scope | Resource | Secret location |
|---|---|---|
| Region | One physical Postgres backup bucket | Provider |
| Region | One backing-provider access-key pair | OpenBao |
| Region | One credential-derivation signing key | OpenBao |
| Organization and region | One versioned SSE-C key | OpenBao |
| Database branch | Prefix, owner, and credential generation | Existing control-plane metadata |
| Database branch | Derived access-key pair consumed by Barman | Kubernetes `Secret` only |

There is no physical bucket, provider credential, or OpenBao access-key entry
per database branch. The Kubernetes `Secret` still exists because the Barman
Cloud Plugin requires an S3-compatible access-key pair when workload identity
is unavailable.

## Object Layout

Each branch owns exactly one prefix:

```text
postgres/{organization_id}/{database_id}/{database_branch_id}/
```

Names use immutable internal IDs rather than customer-selected names. Barman's
`destinationPath` targets this prefix. The Storage API must enforce it instead
of trusting the client configuration.

Prefix enforcement applies to object reads, writes, heads, listings, bulk
deletes, copies, and multipart operations. Listing requests must never be able
to widen their prefix to the bucket root. Copy requests must authorize both
the source and destination.

## Credential Ownership

There are three separate credential classes:

1. **Backing-provider credential** — regional and held only by the Storage API.
2. **Postgres branch credential** — derived, branch-scoped, and accepted only
   by the Storage API.
3. **Recovery credential** — derived, read-only, source-prefix-scoped, and
   short-lived.

Registry credentials remain explicit service credentials because Registry is
a singleton platform workload with a fixed storage target. Postgres cannot use
one literal regional client credential: it represents many customer-controlled
database runtimes and therefore cannot establish request ownership.

An access-key ID may carry the credential kind, branch ID, generation, and key
version. The resolver must load the branch and derive the organization,
database, region, and allowed prefix from control-plane state. Values supplied
in the object key or access-key ID are selectors, not trusted ownership claims.

The secret can be derived using a domain-separated HMAC from the regional
signing key, for example:

```text
secret = HMAC-SHA256(
  regional_signing_key,
  "cplane/postgres-backup/v1" || branch_id || credential_generation || purpose
)
```

The branch's credential generation provides revocation without storing the
secret. Incrementing it invalidates the previous pair. Regional signing-key
rotation requires a version in the access-key ID and an overlap period while
cluster secrets are reconciled.

## SSE-C Ownership

The Storage API selects the SSE-C key from the authenticated branch owner, not
from the requested prefix. Postgres credentials must not be allowed to replace
the platform key with client-supplied SSE-C headers.

The key path is versioned and scoped to an organization and region, for
example:

```text
storage/sse-c/organizations/{organization_id}/{region_id}/{key_version}
```

The organization key is never delivered to CloudNativePG, Barman, cluster
agents, or the backing provider as stored configuration. Storage supplies it
over TLS only on provider operations that encrypt or decrypt object content.

SSE-C is confidentiality defense in depth, not authorization:

- It encrypts object data but not object names or metadata.
- It does not prevent a backing-provider credential from listing or deleting
  objects.
- Losing the SSE-C key makes the affected backups unrecoverable.
- Rotating it requires retaining the old version or rewriting existing
  objects; never replace an active key in place.

Enable provider versioning and object lock where supported. CNPG branch
credentials should not perform fleet-wide retention cleanup; a regional,
durable cleanup worker deletes only an authorized branch prefix after its
retention window.

## Security Boundaries

| Compromise | Expected maximum exposure |
|---|---|
| Database branch or its Kubernetes `Secret` | That branch's backup prefix |
| Organization SSE-C key | Ciphertext for that organization and region, if object access is also obtained |
| Backing-provider credential | Regional object names and availability; object contents remain encrypted |
| Regional signing key | Derived Storage credentials for the region |
| Regional Storage API | Plaintext backups in that region |

The regional Storage API is intentionally a high-trust data-plane boundary.
It must be isolated per region, use workload identity for OpenBao, redact all
credential and SSE-C headers, and never expose backing-provider presigned URLs.

## Why Not One Regional Client Credential?

One shared credential presented to the Storage API contains no organization or
branch identity. Selecting an SSE-C key from the requested object path would
let any holder request another organization's prefix and cause Storage to
decrypt it. Encryption cannot repair missing authorization.

Using a derived branch credential keeps only one root signing secret per region
while preserving independent authorization and revocation. The control-plane
row is not costly fan-out: it is ownership metadata attached to a database
branch that already exists. The expensive fan-out avoided by this design is
physical buckets, provider identities, and OpenBao secret objects.

## Operational Model

- A region is the physical storage and provider-credential blast radius.
- An organization is the default encryption-key boundary.
- A database branch is the runtime, backup-prefix, and access-authority
  boundary.
- Dedicated buckets, KMS keys, VPCs, or cloud accounts remain possible as an
  enterprise isolation tier rather than the default.

The bucket shares provider-level lifecycle configuration and quotas. Per-branch
retention remains a Barman policy plus a durable Storage cleanup job. Capacity,
request rate, and cleanup work must be metered by authenticated owner so one
organization cannot become an unbounded noisy neighbor.

## Storage API Changes

The existing service already separates client and backing-provider credentials
and injects platform SSE-C headers. Implement the remaining behavior in the
shared authenticated Storage API rather than deploying an auth-free instance:

1. Add a Postgres credential resolver backed by a regional signing key and
   database-branch ownership metadata.
2. Add a fixed `base_prefix` and organization SSE-C key reference to the
   resolved storage target.
3. Enforce the prefix across every implemented S3 object and multipart
   operation.
4. Materialize derived credentials directly into the Barman Kubernetes
   `Secret` without persisting their secret value in OpenBao or rendered
   history.
5. Add integration tests proving cross-branch reads, listings, copies, and
   deletes fail closed.

## Alternatives Rejected

- **Bucket per database branch** — creates unnecessary provider control-plane
  resources and lifecycle work.
- **One organization bucket and unrestricted organization credential in every
  branch** — makes a database compromise an organization-wide backup incident.
- **One regional client credential** — cannot establish tenant ownership.
- **Auth-free internal Storage API** — network location is not a sufficient
  authorization boundary for customer-facing database runtimes.
- **Replacing CloudNativePG with Neon Operator** — changes the database storage
  architecture and operational surface without removing the need for secure
  tenant identity.

## Industry Context

Managed database providers generally share control planes and bulk storage
systems while isolating customer-facing database deployments, credentials, and
logical namespaces. PlanetScale exposes isolated branch deployments; Neon uses
project-level compute over a multi-tenant storage system; ClickHouse Cloud uses
stateless compute over shared object storage and offers stronger infrastructure
isolation through higher service tiers. Their exact internal bucket and IAM
layouts are not public contracts.

The C-Plane design follows the same boundary: shared regional infrastructure,
branch-scoped authority, organization-scoped encryption, and optional physical
isolation when a customer requires it.

## References

- [Barman Cloud object-store configuration](https://cloudnative-pg.io/plugin-barman-cloud/docs/next/object_stores/)
- [Amazon S3 SSE-C considerations](https://docs.aws.amazon.com/AmazonS3/latest/userguide/ServerSideEncryptionCustomerKeys.html)
- [Amazon S3 SSE-C request requirements](https://docs.aws.amazon.com/AmazonS3/latest/userguide/specifying-s3-c-encryption.html)
- [PlanetScale Postgres branching](https://planetscale.com/docs/postgres/branching)
- [Neon architecture](https://neon.com/docs/introduction/architecture-overview)
- [ClickHouse Cloud stateless compute](https://clickhouse.com/blog/clickhouse-cloud-stateless-compute)
