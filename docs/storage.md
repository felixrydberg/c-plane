# Storage

Storage is the platform's regional S3-compatible object service. Applications use standard S3 clients and platform-issued credentials; they do not interact with the selected regional backing provider directly.

## Buckets

A bucket is a global logical resource within a project. Its name, region, visibility, credentials, and lifecycle belong to the project.

Every logical bucket has one dedicated real bucket at the configured regional S3 provider. All project branches share that physical bucket while their namespace roots determine which immutable objects each branch can see.

This provides two tenant boundaries: platform authorization and namespace isolation at the Storage API, plus a separate provider bucket for the logical bucket. Provider bucket names and credentials are never exposed. Logical public access is still served by the Storage API; backing buckets remain private.

## Platform encryption

Every bucket has a random platform-managed SSE-C key stored in OpenBao. The Storage API supplies that key only to the regional provider when reading or writing objects. The key is never exposed to applications, persisted in Storage metadata, or written to logs.

This means that leaked R2/S3 provider credentials alone cannot decrypt object content. They may still permit listing or deletion, so provider credentials remain tightly scoped and monitored.

Platform-encrypted transfers pass through the Storage API so it can keep the key secret. The API streams bytes and does not buffer complete objects in memory; direct provider-presigned URLs are not issued for platform-encrypted objects.

## Branching

Storage supports project branching alongside containers and databases.

When a project branch is created, every branch-enabled bucket receives an isolated snapshot of its parent's keys and objects. Creating the snapshot does not copy object data or enumerate keys. Parent and child initially share the same immutable namespace and blobs, then copy only changed namespace paths and newly written object data.

```text
project/main
  database: main
  containers: main
  bucket "uploads": root A

project/feature
  database: feature
  containers: feature
  bucket "uploads": root A  <- shared snapshot
```

If `feature` replaces an object, only that branch points to the new blob. Changes in the parent after the fork are not visible to the child. Deleting a branch does not affect objects still referenced by another branch.

Buckets may also be configured as shared when every project branch intentionally needs the same live namespace. Branch-enabled storage is the default for application data that must stay synchronized with branched database rows.

## Storage API deployment contract

Enabling Storage in a region requires two additional container roles:

1. **Database**: the transactional storage metadata database. It holds namespace trees, branch roots, uploads, and deletion accounting.
2. **API**: the stateless S3-compatible endpoint. It authenticates clients and moves data between clients, metadata, and the regional provider.

Both roles may have multiple replicas. "Two containers" describes their responsibilities, not a single-replica production topology.

## Connections

Applications receive:

```text
endpoint_url
access_key_id
secret_access_key
region
```

Credentials are scoped to an organization, project, and project branch. The logical bucket name stays the same across branches; the credentials select the isolated branch view.

The service is compatible with common S3 clients such as AWS SDKs, AWS CLI, boto3, rclone, and s3cmd for the operations implemented by the platform.

Creating a bucket through the S3 API provisions both the global logical bucket and its dedicated provider bucket. The operation does not succeed until the provider bucket and empty namespace root are ready.

## Upload visibility

The platform records an upload session before an upload begins. This makes long-running and multipart uploads visible as `initiated`, `uploading`, `verifying`, or `committing` even if the client or API process reconnects.

Incomplete uploads do not appear through ordinary S3 `GET`, `HEAD`, or `LIST`. An object becomes visible atomically only after its bytes and metadata have been verified and committed.

## Customer-provided encryption keys

The Storage API supports S3 SSE-C request fields:

- `sse_customer_algorithm`
- `sse_customer_key`
- `sse_customer_key_md5`

The customer key replaces the platform bucket key for that object. It is used for the request and is never persisted or logged. Clients must provide the same key when reading an SSE-C encrypted object. SSE-C availability depends on support from the bucket's regional backing provider.

## Deletion

The platform tracks references between branch roots, namespace nodes, and immutable blobs. Removing an object or branch releases only data that is no longer referenced anywhere.

Large branch or bucket deletions can require many provider delete operations. Namespace removal is immediate; physical cleanup is durable, observable, and retried until complete.

## Implementation

The internal design and supported S3 operation surface are documented in [Storage Service Implementation](services/s3.md).
