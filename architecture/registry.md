# Container registry architecture

## Model

C-Plane runs one shared CNCF Distribution service for all organizations. It is
a separate container in the control-plane Compose stack; it does not run in
Kubernetes and does not store image data in the API container or Postgres.

Each organization owns one repository namespace:

```text
registry.platform.dev/{organization-slug}/{repository}:{tag}
```

Repositories are created explicitly in C-Plane. Postgres stores their identity
and access grants. Distribution stores manifests, upload state, and layers in
one S3-compatible bucket. Blobs are content-addressed and deduplicated across
the registry, while repository metadata remains namespaced by organization.

## Components

```text
CI or container runtime
  -> Distribution
  -> C-Plane registry token endpoint
  -> identity database
  -> Distribution
  -> Storage service
  -> assigned S3-compatible provider
```

- **Distribution** implements the OCI Registry v2 push and pull protocol.
- **C-Plane API** authenticates organization credentials and signs short-lived
  repository access tokens.
- **Identity database** stores repositories, dedicated registry access tokens,
  per-token pull/push grants, and organization membership. Only token hashes
  and display prefixes are retained.
- **Storage service** authenticates Distribution's platform service
  credential, resolves the global registry bucket, and supplies its encryption
  key when proxying provider requests.
- **S3-compatible storage** durably stores all registry content behind Storage.

## Authentication and tenant isolation

Distribution does not model tenants. The C-Plane token endpoint provides the
tenant boundary:

1. The client requests a repository operation from Distribution.
2. Distribution responds with a Bearer challenge naming the public C-Plane
   token endpoint and required repository scope.
3. The client authenticates to that token endpoint with the organization slug
   as username and a dedicated registry access token as password.
4. C-Plane hashes and resolves the registry token, verifies the username matches
   its organization, loads the repository grant, and intersects the requested
   actions with that grant. It only grants existing repository names beginning
   with that organization's slug and signs a short-lived HS256 JWT.
5. The client retries the repository operation with the JWT. Distribution
   verifies it and permits only its repository and actions.

Registry access tokens cannot authenticate to normal control-plane API routes,
and control-plane API keys cannot authenticate to Distribution.
Registry-wide scopes such as catalog access are never issued, preventing one
organization from listing another organization's repositories. Credentials are
limited to the repositories selected when they are created.

## Push and pull paths

For a CI push, the pipeline logs in, builds an image tagged inside its
organization namespace, and pushes it to Distribution. Distribution validates
the push token and writes the OCI data through Storage to the configured
provider.

For a pull, Distribution validates a pull token and reads registry metadata and
layers through Storage. Workloads should deploy immutable image digests rather
than mutable tags.

## Keys and configuration

The API and Distribution receive the same base64url-encoded 256-bit
`REGISTRY_TOKEN_SECRET`. The API signs HS256 tokens with it; Distribution
writes an ephemeral symmetric JWKS under `/run` at startup. A dedicated
platform S3 service access-key pair authenticates Distribution to Storage, and
a second pair is reserved for garbage collection.
The singleton `registry_storage` row stores the normal credential and bucket
assignment. The disposable `registry_maintenance` row stores the GC credential
ID and current maintenance state. OpenBao stores each secret keyed by its
public access-key ID at `platform/s3/access-keys/{access_key_id}`.

The registry bucket has its own random 256-bit SSE-C key at
`storage/sse-c/{registry_storage_id}`. Storage supplies this key to the backing
provider; Distribution never receives it or the provider credentials. The key
belongs to the global registry bucket rather than an organization so shared
content-addressed layers remain readable across organization namespaces.

Its runtime configuration lives in `packages/registry/config.yml`;
installation secrets provide the public hosts, JWT secret, and Storage
access-key pair. Garbage collection uses the auth-free
`packages/registry/config-gc.yml`, so Worker does not receive the JWT secret.

## Operational boundaries

- Postgres owns repository identity and grants. Distribution owns manifests,
  tags, layers, and upload state.
- Manifest deletion makes blobs eligible for collection. The control plane
  queues a `registry_gc` job in Postgres; horizontally scalable workers claim
  named queues with `FOR UPDATE SKIP LOCKED` and run Distribution's official
  collector. API token grants and Storage permissions make the Registry
  read-only while the shared maintenance state is active.
- Provider mirroring is deferred to the generic Storage design; the registry
  initially uses one authoritative provider.
- Quotas, organization deletion cleanup, and private external-registry
  credentials are not part of the initial implementation.

See `docs/services/registry.md` for setup and CI commands.
