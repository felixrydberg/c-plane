# Durability

Clusters are disposable. All state that must survive cluster loss lives in object storage or a managed database. The durability layer is at the region, not the cluster.

## Object Storage Backing

- Each region defines one storage backend. The backend is an implementation detail — users never see it.
- Supported backends: AWS S3, Cloudflare R2, Ceph RGW, MinIO (or any S3-compatible endpoint).
- R2 is the natural default for cloud-hosted regions: 1M bucket limit out of the box, no account management needed.
- AWS S3 defaults to 10k buckets per account but can be raised to 1M via a support request. Operators manage this themselves.
- Self-hosted minimum requirement: any reachable S3-compatible endpoint. No effective bucket limit.
- Backend credentials are stored at the region level and never exposed externally.

## What Lives Here

- CloudNativePG WAL archives and base backups.
- Database branch snapshots (used for LSN-based recovery).
- Future: cluster state snapshots for fast reprovisioning.

## Rules

- Cluster agents write to the region's configured backend using injected credentials.
- Changing a region's backend requires a data migration — it is not a live operation.
- No cluster owns object storage. Destroying a cluster does not destroy data.
