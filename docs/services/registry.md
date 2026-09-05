# Managed Registry

Each organization explicitly activates one Managed Registry in one immutable
region. Activation synchronously creates one tenant foundation bucket, one
tenant-scoped S3 credential, and one read/write `bucket_grant`. The grant stays
an infrastructure authorization record keyed by credential and foundation
bucket; the resolver derives the fixed logical bucket name `registry` through
`managed_registry`.

Repository and Registry access-token creation require activation. Registry
storage is deleted only with the organization. Metadata deletion removes the
foundation bucket row, which uses the normal `foundation_bucket_delete` worker
job for asynchronous provider cleanup.

## Request path

The Registry service is a custom Go binary built with CNCF Distribution
v3.1.1. Distribution uses a complete delegating storage driver:

```text
OCI client -> Registry -> standard Storage S3 endpoint -> foundation bucket
                         SigV4 credential + logical bucket "registry"
```

The service never receives a physical provider bucket name, provider
credential, SSE key, or Registry-specific Storage mode. Its per-organization
S3 driver cache is keyed by organization ID, expires after 15 idle minutes by
default, coalesces concurrent misses, and replaces entries when
`storage_revision` changes.

Registry bearer tokens contain an immutable `organization_id`. The outer HTTP
middleware verifies HS256, issuer, audience, organization metadata, and that
every repository scope begins with the current organization slug. Distribution
uses the registry's C-Plane access controller for the same HS256 token so
repository actions are checked without the upstream JWKS controller, which
expects public-key-compatible signing keys. Catalog access is disabled.

## Garbage collection

Every managed Registry has an `active` or `maintenance` status. While an
organization is in maintenance, all authenticated Distribution writes for that
organization return an OCI-shaped `503`; reads (pulls, tag listing) stay
available while other organizations continue
normally. The API also blocks Registry mutations while it is not active.

Organization admins and owners can queue a cleanup with
`POST /api/organization/{organization_id}/registry/garbage-collection`.

The worker switches only the target organization's Registry to maintenance and
calls the Registry over the internal service network:

```text
registry serve /etc/distribution/config.yml
POST /internal/organizations/{organization_id}/garbage-collection
```

The endpoint requires the C-Plane service token, resolves current bucket
credentials through the internal API, waits for that organization's in-flight
requests to finish, and runs Distribution mark-and-sweep against its foundation
bucket. It is not exposed by ingress. The worker always restores `active`,
records success or failure. Upstream global upload purging remains disabled
because it has no tenant context.

## External registries

C-Plane separately stores reusable pull credentials for Docker Hub, GitHub
Container Registry, GitLab Container Registry, Google Artifact Registry, and
AWS Elastic Container Registry. Tokens are stored in OpenBao and are never
returned by the API.
