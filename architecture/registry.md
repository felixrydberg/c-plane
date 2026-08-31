# Container Registry architecture

## Model

C-Plane runs one shared CNCF Distribution service, but every organization
explicitly activates its own managed Registry in one immutable region. Each
managed Registry owns one tenant credential and one foundation bucket.

```text
registry.platform.dev/{organization-slug}/{repository}:{tag}

organization
  -> managed_registry
  -> credential -> bucket_grant -> foundation bucket
```

The shared Registry process is stateless. Organization-specific storage is
selected from the authenticated request rather than from process-wide S3
configuration.

## Request path

1. Distribution challenges the client for a C-Plane Registry token.
2. The API validates the Registry access token and repository grant, then signs
   a short-lived HS256 token containing the immutable organization ID.
3. The Registry middleware and C-Plane Distribution access controller validate
   that token; the middleware resolves the organization's current managed
   Registry metadata from the API and the access controller checks repository
   actions.
4. The delegating storage driver selects a cached Distribution S3 driver for
   that organization.
5. The driver signs ordinary S3 requests with the organization's tenant
   credential and the logical bucket name `registry`.
6. Storage resolves the credential through `bucket_grant`, derives the logical
   name through `managed_registry`, and proxies to the foundation bucket's S3
   provider with its SSE-C key.

Distribution never receives a provider credential, physical bucket name, or
SSE-C key. The foundation bucket is not shared with another organization.

## Isolation

- Registry access tokens cannot authenticate to control-plane API routes.
- Control-plane API keys cannot authenticate to Distribution.
- Repository scopes must begin with the organization slug resolved from the
  immutable organization ID in the Registry JWT.
- Catalog access is disabled.
- A Registry credential has exactly one read/write grant to its foundation
  bucket. Grant uniqueness remains `(credential_id, bucket_id)`.

## Lifecycle and garbage collection

Activation is synchronous and idempotent. It creates
the bucket SSE secret, foundation bucket, tenant credential, grant, and
`managed_registry` row in one organization-scoped operation. The region cannot
be changed later and there is no standalone Registry deletion endpoint.
Organization deletion revokes the grant and credential, deletes Registry
metadata and secrets, and queues the existing asynchronous physical bucket
cleanup by deleting the foundation bucket row.

Each `managed_registry` is either `active` or `maintenance`. Distribution
resolves that status for every authenticated request and returns an OCI-shaped
`503` for pulls, pushes, and deletes while the affected organization is in
maintenance; other organizations remain available.

Activation enables a daily garbage-collection schedule at 03:00 UTC. The
schedule stores local wall-clock time plus timezone so it stays at night across
daylight-saving changes. An
organization admin or owner can change or disable the schedule and queue an
immediate run through the organization API. Each run uses an organization
dedupe key, switches only that Registry to maintenance, and calls the Registry's
service-authenticated internal endpoint. The Registry waits for that
organization's in-flight requests to finish, then runs Distribution
mark-and-sweep directly against its foundation bucket. Completion restores
`active`, records the result, and schedules the next daily run.

## Runtime

`packages/registry` builds a Go 1.25 binary pinned to CNCF Distribution v3.1.1.
It registers an outer request middleware and a complete delegating StorageDriver
without forking Distribution. Its bounded, concurrency-safe driver cache is
keyed by organization ID and invalidated by the managed Registry storage
revision.

See `docs/services/registry.md` for activation and client usage.
