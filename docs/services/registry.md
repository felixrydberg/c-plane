# Container Registry

## v1 — External Registry

In v1 users bring a pre-built image from any OCI-compatible registry. The platform does not build images.

- User provides an image reference: `registry.example.com/org/app:sha256-abc123` or `:tag`.
- Platform recommends digest pinning over tags for reproducible deploys — tags are mutable, digests are not.
- If the registry requires auth, the user provides pull credentials (username + token/password). Stored as a secret, injected as `imagePullSecrets` on the `Deployment`.
- Pull credentials are scoped to a project and reusable across deployments in that project.
- Supported registries: Docker Hub, GitHub Container Registry (ghcr.io), Google Artifact Registry, AWS ECR, any private registry with standard Docker auth.

### ECR note

AWS ECR credentials expire every 12 hours. For ECR, the cluster agent must refresh the `imagePullSecret` on a schedule using the stored AWS credentials. This is the one registry-specific edge case.

## v2 — Platform Registry (Future)

The platform runs a single global [Distribution](https://github.com/distribution/distribution) registry backed by R2. Push once, deploy to any region — no per-region push or cross-region replication needed. R2 has no egress fees, so global pulls are bandwidth-bound only.

- **Implementation**: Distribution registry with the built-in S3 storage driver.
- **Backend config**: `regionendpoint` + `forcepathstyle: true` makes it work identically against R2, Ceph RGW, and MinIO — the same driver config, pointing at whichever backend the region uses.
- **One registry bucket per region**: a dedicated platform-managed S3 bucket (separate from user buckets). Distribution owns the key layout inside it entirely.
- **Repository namespacing**: `registry.<region>.platform.dev/{org}/{project}:{tag}`. OCI repository names provide logical isolation — no prefix tricks needed.
- **Pull traffic**: Distribution redirects blob downloads to presigned S3 URLs. The registry process only serves manifest API calls and auth tokens. Blob bytes go directly from S3 to the client — no bandwidth cost to the registry pods.
- **Garbage collection**: built into Distribution (`registry garbage-collect`). Runs as a scheduled job. Removes unreferenced layers from the S3 bucket.
- **Auth**: Distribution's token auth spec. Platform issues short-lived JWT tokens scoped to a repository on push/pull. The registry validates them.
- **Required S3 permissions** on the registry bucket:
  - Bucket level: `ListBucket`, `GetBucketLocation`, `ListBucketMultipartUploads`
  - Object level: `GetObject`, `PutObject`, `DeleteObject`, `ListMultipartUploadParts`, `AbortMultipartUpload`
- **In-cluster layer caching**: [Spegel](https://github.com/spegel-org/spegel) is a candidate for reducing repeat pull traffic. It runs as a DaemonSet and turns the cluster into a p2p mirror — if any node already has a layer, it serves it to other nodes directly without hitting the registry. Worth evaluating when the platform registry is implemented.

## Build Pipeline (Future)

Build-on-push (Dockerfile in repo → platform builds and deploys) is explicitly out of scope for v1 and v2. It is a significant surface area and adds a build execution environment that is separate from the deployment platform. It can be added later without changing the deployment model — the output is still just an image reference.
