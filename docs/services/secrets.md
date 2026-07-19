# Secrets Service

This document defines how C-Plane stores and delivers secrets with a separate
OpenBao deployment. OpenBao is the source of truth for secret values. C-Plane
Postgres stores ownership, names, references, lifecycle state, and OpenBao
version metadata only.

## Goals

- Keep S3 provider credentials out of Postgres.
- Store the platform-managed SSE-C bucket keys described in
  [S3 service implementation](s3.md#platform-managed-sse-c-keys) in OpenBao.
- Store project and registry secrets in OpenBao and provide them to Kubernetes
  workloads through the OpenBao Kubernetes integration.
- Prevent tenant-facing APIs, database dumps, logs, and ordinary compute
  containers from receiving unrelated secrets.
- Make rotation, deletion, backup, and recovery explicit.

OpenBao is not exposed to users. The dashboard and public API call the control
plane. Kubernetes workloads access only their allowed paths through the
OpenBao Kubernetes integration.

## Deployment boundary

Run OpenBao as a separate persistent service in the platform's management or
regional infrastructure:

```text
                         TLS + workload identity
                 +------------------------------------+
                 |                                    v
Dashboard --> Control plane --> OpenBao <-------- Storage API
                    |              ^                 |
                    |              |                 | bucket SSE-C key
                    v              |                 v
             Kubernetes API        |          Regional S3 provider
                    |              |
                    v              |
             Compute workload -----+
             (OpenBao Agent Injector)
```

The first deployment may be one OpenBao container with a persistent volume.
That is suitable for development or a deliberately accepted single-node
failure domain only. Production should use three OpenBao nodes with
integrated Raft storage across failure domains. OpenBao recommends integrated
storage for most deployments and documents Raft as production-ready with HA:
[storage configuration](https://openbao.org/docs/configuration/storage/),
[integrated storage](https://openbao.org/docs/concepts/integrated-storage/).

Use the following minimum deployment properties:

- Pin an explicitly reviewed OpenBao image version; do not use `latest`.
- Use persistent SSD-backed storage for the Raft data directory.
- Enable TLS on the API listener. Do not run dev mode in production.
- Keep the API and Raft cluster ports on private network paths.
- Provide a backup/snapshot schedule and a tested restore procedure.
- Use an auto-unseal mechanism backed by the platform KMS when available.
  Otherwise, document manual Shamir unseal ownership and keep key shares
  outside the cluster.
- Do not put a root token, unseal key, or recovery key in an application
  Secret, image, repository, or environment variable.

OpenBao starts sealed and cannot serve normal requests until unsealed. Auto
unseal reduces restart-related operational work; it does not remove the need
to protect recovery material. See [seal and unseal](https://openbao.org/docs/next/concepts/seal/).

## Storage model

Enable one explicit KV version 2 mount for C-Plane, for example `cplane/`.
Do not depend on the default `secret/` mount or dev-mode defaults.

The logical paths below are relative to the `cplane` mount. The underlying KV
v2 API paths include `cplane/data/` for values and `cplane/metadata/` for
metadata.

| Secret | Logical path | Payload |
| --- | --- | --- |
| Regional S3 provider | `platform/s3/providers/{provider_id}` | `access_key_id`, `secret_access_key`, optional `session_token` |
| Platform S3 service credential | `platform/s3/service-credentials/{credential_id}` | `secret_access_key` |
| Platform bucket encryption key | `storage/sse-c/{bucket_id}` | `key` containing 32 random bytes encoded as base64 |
| Project secret | `projects/{project_id}/secrets/{secret_id}` | `value` |
| Registry pull credentials | `projects/{project_id}/registry/{credential_id}` | registry username/token or equivalent |
| Platform signing material | `platform/{service}/{key_name}` | service-specific secret material |

The bucket path intentionally matches the existing S3 design. Generate the
bucket key with the operating system CSPRNG; never derive it from a bucket
name, project ID, or user input.

Use opaque UUIDs in paths. Do not put organization names, project names,
secret names, provider URLs, or credentials in paths because paths appear in
policies and audit records.

KV v2 provides versioning. Keep the current version in control-plane metadata
when a deployment must reproduce an exact historical configuration. Use CAS
on writes where the caller knows the expected version. OpenBao documents the
KV v2 semantics and API at [KV v2](https://openbao.org/docs/secrets/kv/kv-v2/).

## Database changes

The existing schema already has the right reference shape for project
secrets: `project_secret` identifies a named secret and
`project_container_version.env_secret_refs` stores IDs rather than values.
Change the value-bearing columns as follows:

- `s3_providers`: retain provider type, endpoint, provider region, active
  state, and timestamps. Remove `access_key_id`,
  `secret_access_key_encrypted`, and `session_token_encrypted` from the
  secret-bearing model; all three belong in the provider secret payload.
- `project_secret_version`: replace `value_encrypted` with
  `openbao_version` (or an equivalent integer) and retain the application
  version, timestamps, and ownership fields.
- `project_secret`: retain the project/name/lifecycle metadata. Its ID is the
  stable OpenBao path component.
- `project_container_version.env_secret_refs`: keep the existing map of
  environment variable names to `project_secret.id` values. It must never
  contain secret values.
- `bucket`: no encryption key column is needed. The bucket UUID derives the
  OpenBao path in `storage/sse-c/{bucket_id}`.
- `registry_storage`: store the Distribution service name, access-key ID,
  provider assignment, logical bucket name, and physical bucket name. Its UUID
  is the stable OpenBao path component. The row has no organization or project
  owner.

The database remains the source of truth for tenant authorization and
resource relationships. OpenBao remains the source of truth for secret bytes.

## Access policies and identities

Use separate Kubernetes service accounts and OpenBao roles for each trust
boundary. The control plane should not use the same identity as the Storage
API or Kubernetes workload.

| Identity | Required access |
| --- | --- |
| Control plane writer | Create/update/delete project, registry, and provider paths; no list-all or broad read access |
| Control plane S3 resolver | Read the exact `platform/s3/service-credentials/{credential_id}` selected by non-secret Postgres metadata; no list access |
| Kubernetes workload ServiceAccount | Read only the project/registry paths needed by that workload |
| Storage API | Read `storage/sse-c/*` and the provider path for its own region; no project-secret access |
| Operator/bootstrap job | Configure mounts, auth, policies, rotation, and recovery; not used by services |

For KV v2, grant policies against `data/` paths and grant only the minimum
metadata access needed for version checks. Do not grant `list` at a parent
path unless the service genuinely needs enumeration. Do not give tenant users
OpenBao tokens.

Authenticate Kubernetes workloads with the OpenBao Kubernetes auth method and
short-lived service-account tokens. Bind each OpenBao role to exact service
account names and namespaces, and keep token TTLs short enough that revocation
is operationally useful. OpenBao's primary guidance is in the
[Kubernetes auth method](https://openbao.org/docs/next/auth/kubernetes/).

This means Kubernetes auth plus Agent Injector; it does not mean the separate
OpenBao Kubernetes secrets engine. That engine generates Kubernetes
service-account tokens and is not needed for application secret delivery.

## Write flows

### Project or registry secret

1. Authenticate the request and verify organization access.
2. Verify project access and validate the secret name and environment key.
3. Generate the secret ID and write the value to its OpenBao KV path.
4. Record only the secret metadata and OpenBao version in the Postgres
   transaction.
5. Return the secret name and version metadata, never the value.

Do not attempt a distributed transaction between Postgres and OpenBao. If the
OpenBao write succeeds and the database transaction fails, retry the metadata
write idempotently and let a small reconciliation check find orphaned paths.
An orphaned value is inaccessible without the corresponding database
relationship and policy; it should still be deleted after the repair window.

### S3 provider credentials

1. An operator submits provider configuration through a privileged control
   plane path.
2. Store all provider credentials in `platform/s3/providers/{provider_id}`.
   Providers are configured before a region references them, so the stable
   provider ID—not the mutable region assignment—forms the path.
3. Store only provider metadata and the stable provider ID in Postgres.
4. Give the regional Storage API read access to the provider path for its
   region.
5. On rotation, write a new KV version, reload the Storage API credentials,
   and deactivate the old provider credential at the external provider only
   after the new credential has been verified.

### Platform S3 service credential

An installation may provision an internal service, such as Distribution, as
an S3 client:

1. Generate one access-key ID, one cryptographically random secret access key,
   and one opaque credential ID.
2. Store the access-key ID and storage assignment in `registry_storage`.
3. Store the secret at
   `platform/s3/service-credentials/{credential_id}` in OpenBao.
4. Inject the same generated access-key pair into the service through the
   installation secret mechanism.
5. Grant the credential only its platform-owned logical bucket.

The control plane resolves the presented access-key ID to the OpenBao secret;
Storage verifies the request's SigV4 signature. A different injected secret
therefore fails authentication rather than selecting a fallback identity.
Backing-provider credentials and the bucket encryption key are never injected
into the client service.

### Platform SSE-C bucket key

When a bucket is created:

1. Reserve the bucket ID and validate organization, project, and region access.
2. Generate a 256-bit key with the OS CSPRNG.
3. Write it once to `storage/sse-c/{bucket_id}` using KV v2 CAS.
4. Provision the backing provider bucket.
5. Commit the logical bucket metadata and namespace root only after both the
   provider bucket and OpenBao key exist.

If provisioning fails, leave the logical bucket unavailable and clean up the
provider bucket and OpenBao path through the existing retry/reconciliation
workflow. Never return the key to the control plane response, database,
client, or logs.

## Kubernetes secret delivery

Kubernetes is the compute-layer integration point. Use the OpenBao Agent
Injector installed by the OpenBao Helm chart. The injector adds an OpenBao
Agent init/sidecar to selected workloads; the agent authenticates with the
workload's Kubernetes ServiceAccount and renders only the annotated secret
paths into an ephemeral in-memory volume. OpenBao describes Agent Injector as
the more mature Kubernetes integration and notes that it avoids durable secret
storage outside OpenBao: [Kubernetes integrations](https://openbao.org/docs/2.4.x/platform/k8s/),
[OpenBao Agent](https://openbao.org/docs/agent-and-proxy/agent/).

The compute controller should:

1. Read the pinned container version and its `env_secret_refs` from Postgres.
2. Verify organization, project, branch, and deployment ownership.
3. Create or select a dedicated Kubernetes ServiceAccount for the workload.
4. Set the pod annotations consumed by the OpenBao Agent Injector, including
   the OpenBao role and the exact secret paths required by that deployment.
5. Start the workload only after the agent has authenticated and rendered the
   required values.

Use a dedicated ServiceAccount per project or deployment boundary; never use
`default`. Bind that ServiceAccount to an OpenBao Kubernetes-auth role whose
policy permits only the relevant project paths. The compute controller owns
the mapping from `project_secret.id` to the OpenBao path; tenant input must
not supply arbitrary OpenBao paths or roles.

For the current container contract, render a single environment file such as
`/openbao/secrets/env` from the KV values and start the application through a
small, fixed entrypoint that reads that file before `exec`. If the application
already supports secret files, mount individual files instead. Do not sync
the values into a Kubernetes Secret by default: that creates a second durable
secret store and broadens the Kubernetes RBAC blast radius.

OpenBao Agent can refresh KV v2 values periodically, but a process that copied
values into its environment will not see updates. Treat secret changes as a
deployment restart/reconciliation event unless the application explicitly
supports file-based reload. Agent templating supports rendering static KV
values and process-supervisor environment variables; the latter is currently
documented as public beta, so use file rendering plus the existing fixed
entrypoint for the first implementation:
[Agent templates](https://openbao.org/docs/agent-and-proxy/agent/template/),
[process supervisor](https://openbao.org/docs/agent-and-proxy/agent/process-supervisor/).

The compute workload receives the value it needs, but not a reusable
platform-wide token or permission to read other projects' secrets. Do not put
resolved values in deployment events, API responses, rendered YAML previews,
metrics, traces, or ordinary logs.

## Read paths for storage

For every provider operation involving a platform-encrypted object, the
Storage API:

1. Resolves the bucket and verifies the request's organization/project/branch
   access.
2. Reads `storage/sse-c/{bucket_id}` with its regional OpenBao identity.
3. Uses the key only in memory to send the provider SSE-C headers.
4. Computes the required key MD5 in memory.
5. Drops the key after the provider operation.

This preserves the defense-in-depth property in `s3.md`: provider credentials
alone cannot decrypt platform-encrypted objects, while OpenBao access alone
does not grant S3 authorization or object namespace access.

## Rotation and deletion

- Project secret updates create a new KV v2 version and a new application
  version. Existing deployments keep their pinned resolved value until they
  are redeployed or explicitly refreshed.
- Provider credential rotation is a two-phase external operation: add and
  verify the new credential, update OpenBao, reload consumers, then revoke the
  old credential.
- Bucket SSE-C keys are not rotated in place. Rotation requires a deliberate
  object re-encryption/migration design; until then, treat the bucket key as
  immutable.
- Deleting a project or registry secret first removes references and prevents
  new deployments, then soft-deletes the OpenBao value. Permanently destroy
  old KV versions only after the configured retention period.
- Delete a bucket key only after the provider data and namespace metadata are
  irreversibly deleted and recovery requirements have expired.

## Observability and recovery

Configure at least one audit device before enabling application access, and
prefer two independent destinations. Keep raw logging disabled so sensitive
request and response values remain HMAC-protected. OpenBao notes that requests
can fail when all audit devices are unavailable, so monitor audit delivery as
part of service health. See [audit devices](https://openbao.org/docs/next/audit/)
and [declarative audit configuration](https://openbao.org/docs/configuration/audit/).

Redact authorization headers, OpenBao tokens, provider credentials, SSE-C
keys, customer encryption keys, and presigned query strings from all C-Plane
logs and traces.

Back up the encrypted OpenBao Raft state and the server configuration/management
material separately. A restore is not complete until the cluster is unsealed,
policies and auth methods are present, a test workload can authenticate, and a
test Storage API can read a non-production bucket key. OpenBao's recovery
guidance explicitly treats HA and backups as separate requirements:
[storage backups](https://openbao.org/docs/concepts/storage/).

## Initial implementation slice

Keep the first implementation small:

1. Deploy one non-dev OpenBao instance with persistent Raft storage for local
   development and a three-node Raft deployment for production.
2. Enable the explicit KV v2 mount and configure TLS, Kubernetes auth, audit,
   and the three service policies above.
3. Move S3 provider credentials and project secret values out of the encrypted
   Postgres columns.
4. Install and configure the OpenBao Agent Injector, Kubernetes auth method,
   workload ServiceAccounts, and per-project/workload OpenBao roles.
5. Have the compute controller add injector annotations and render the
   existing container environment contract from an ephemeral secret file.
6. Wire the Storage API to read provider credentials and bucket SSE-C keys.
7. Add one end-to-end test that proves a tenant cannot read another tenant's
   secret, a workload cannot read another project's secret, and the Storage
   API cannot read project secrets.

Do not add dynamic S3 credentials, secret templating, per-tenant OpenBao
namespaces, or a general-purpose secret broker until a real requirement needs
them. KV v2 plus narrow policies covers the current C-Plane flows.
