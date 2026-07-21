# Registry operations

## Garbage collection

Deleting a repository removes its manifests. Shared, content-addressed layers
must not be deleted by S3 prefix; Distribution's garbage collector determines
which blobs are no longer referenced.

Run collection from the private control-plane Overview page:

1. Select **Queue garbage collection** and confirm.
2. The control plane inserts a `registry_gc` job and changes the shared
   `registry_maintenance` state to `queued` in one Postgres transaction.
3. A worker claims the `maintenance` queue and drains for the configured
   Registry token lifetime. New write tokens are rejected immediately, while
   the normal Registry storage credential can finish writes during the drain.
4. After the drain, storage blocks the normal Registry credential and enables
   the dedicated GC credential. The worker runs
   `registry garbage-collect /etc/distribution/config.yml`.
5. The worker restores writes after success or failure. Pulls remain available
   throughout; untagged and digest-only manifests are preserved.

The current phase is shared by every API and control-plane instance. Customer
dashboards poll it every five seconds. Storage permits the normal Registry
credential to write while the phase is `idle`, `queued`, or `draining`, and
permits the dedicated GC credential to write only while the phase is
`collecting`.

Write-token requests receive `503 Service Unavailable` during maintenance.
`REGISTRY_TOKEN_TTL_SECONDS` controls both token lifetime and the drain period;
it defaults to Distribution's 60-second compatibility minimum.

Token expiry prevents an old token from starting another request after the
drain. It does not cancel a request that Distribution already accepted, so the
drain is defense in depth rather than a replacement for Distribution's native
read-only restart requirement.

Interrupted uploads under `_uploads` are not content-addressed blobs and are
not removed by the garbage-collection command. Distribution's separate upload
purger removes old orphaned upload directories; its defaults are enabled, a
seven-day age, and a 24-hour interval.

Worker jobs use leases and at-least-once processing. If a worker exits, another
replica can reclaim the job after the lease expires. The Registry subprocess is
killed if its worker loses the lease.

If the garbage-collection command returns an error, the worker records it,
restores Registry writes, and leaves the failed job in the queue history. Fix
the reported problem and queue a new run from the control plane. If a worker or
dependency is unavailable, the job and non-idle maintenance phase remain in
Postgres; restart the dependency or any worker replica and let lease recovery
continue the job. Do not clear the maintenance row manually while a worker may
still own its lease.

## Worker queues

`worker_job` is the durable generic queue. `queue_name` isolates workloads and
`job_type` selects the handler. Worker replicas claim rows with
`FOR UPDATE SKIP LOCKED`, so replicas process different jobs without a central
coordinator.

Configure a worker with comma-separated queues and local concurrency:

```text
WORKER_QUEUES=maintenance,cluster-state
WORKER_CONCURRENCY=4
```

To add a job, enqueue its type and payload in `worker_job`, then add one match
arm in the worker dispatcher. A partial unique index on `(queue_name,
dedupe_key)` prevents duplicate active jobs when a producer supplies a key.
