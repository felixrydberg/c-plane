## Plan: Cluster Agent Bootstrap MVP

Deliver an end-to-end bootstrap MVP for cluster onboarding with a new Go agent package and Rust control-plane gRPC stream. The approach is to extend existing infrastructure schema/routes for join credentials, add Rust gRPC bootstrap + stream services, scaffold a Go agent that performs join/exchange + heartbeat stream, and wire local Docker/dev tooling. This keeps control-plane authority centralized while introducing the minimum runtime path required by the specification.

**Steps**
1. Phase 0 - Baseline and contracts: define MVP protocol boundaries and lifecycle subset (*blocks all later steps*). Confirm Phase 1 lifecycle states used in code paths are `pending -> bootstrapping -> healthy` with `offline/draining/removed` accepted but not fully automated.
2. Phase 1 - Shared protocol foundation (*blocks Rust + Go implementation*): add proto package under `packages/proto` with v1 service/messages for bootstrap exchange, heartbeat, and desired/observed state stream skeleton. Include basic versioning/package conventions for forward compatibility.
3. Phase 2 - Data model for bootstrap identity (*parallel substeps where noted*):
   - 2a. Extend infrastructure schema in `packages/ui-shared` with join-credential table and cluster status enum expansion (*parallel with 2b*).
   - 2b. Add migration SQL in `packages/ui-shared/drizzle` and schema exports/index wiring (*parallel with 2a*).
   - 2c. Add optional cluster-agent identity table for post-bootstrap credential material and last-rotation metadata (*depends on 2a/2b*).
4. Phase 3 - Control-plane issuance APIs (Nuxt server) (*depends on 2*): implement authenticated issuance endpoints in `packages/ui-studio/server/api/infrastructure/clusters` for creating/reissuing short-lived single-use join credentials, returning raw secret once, storing only hash, and auditing issuance/revocation events.
5. Phase 4 - Rust gRPC server bootstrap path (*depends on 1 and 2*):
   - 4a. Add gRPC/protobuf build pipeline (`build.rs`, tonic/prost deps, generated module integration) in `packages/c-plane`.
   - 4b. Add gRPC service module and bootstrap handler to validate join credential (hash match, unused, unexpired), atomically mark used, issue runtime identity, and transition cluster to `bootstrapping/healthy` appropriately.
   - 4c. Add bidirectional stream endpoint skeleton for heartbeat + desired-state placeholder messaging.
6. Phase 5 - Go agent package scaffold (*depends on 1; partially parallel with 4*): create `packages/c-plane-agent` with `go.mod` and root `go.work` integration, config/env loading, structured logging, and gRPC client implementation for bootstrap then persistent stream reconnect loop.
7. Phase 6 - Agent reconciliation MVP behavior (*depends on 4c + 5*): implement minimal no-op reconciler that reports observed state/health/capacity heartbeat and accepts desired-state placeholder messages without Kubernetes mutation yet.
8. Phase 7 - Tooling and runtime wiring (*depends on 4 + 5*): add `Dockerfile.go`, update `docker-compose.dev.yml` with agent service, expose gRPC port/config in Rust service env, and ensure local developer startup path is documented.
9. Phase 8 - Security hardening pass for MVP (*depends on 3, 4, 5*): enforce TTL + single-use semantics, redact secrets from logs, add explicit scope/role checks for cluster identity, and ensure clusters cannot touch tenant data paths.
10. Phase 9 - Verification and acceptance (*depends on all prior steps*): run schema/type checks, Rust tests, Go unit tests, and an end-to-end local flow (issue credential -> agent bootstrap -> stream heartbeats -> cluster status update).

**Relevant files**
- `specification/cluster-agent.md` — Source specification for behavior, lifecycle, and trust boundaries.
- `packages/ui-shared/schema/clusters/schema.ts` — Existing cluster model and status enum to extend.
- `packages/ui-shared/drizzle/0000_tearful_unicorn.sql` — Existing infrastructure migration baseline.
- `packages/ui-shared/schema/index.ts` and `packages/ui-shared/schema/all.ts` — Schema export wiring.
- `packages/ui-studio/server/api/infrastructure/clusters/index.post.ts` — Existing cluster creation endpoint pattern to extend.
- `packages/ui-studio/server/api/infrastructure/clusters/[cluster_id].patch.ts` — Status mutation/validation pattern reference.
- `packages/ui-shared/utils/event-types.ts` — Audit/event taxonomy for issuance/bootstrap events.
- `packages/c-plane/Cargo.toml` — Rust dependency and build-script integration point for tonic/prost.
- `packages/c-plane/src/main.rs` — Server startup; add gRPC listener alongside existing HTTP.
- `packages/c-plane/src/routes/mod.rs` — Existing HTTP routes; keep health route and avoid regressions.
- `packages/c-plane/src/state/mod.rs` — DB access/state wiring reused by gRPC handlers.
- `packages/c-plane/src/middleware/auth.rs` — Existing hash/auth context patterns to reuse for credential validation.
- `packages/c-plane-agent/go.mod` — New Go module for agent binary.
- `go.work` — Root Go workspace file to include `packages/c-plane-agent`.
- `Dockerfile.rust` — Add protobuf build requirements as needed.
- `Dockerfile.go` — New image build for Go agent.
- `docker-compose.dev.yml` — Local orchestration for control-plane + agent integration.

**Verification**
1. Schema validation: run drizzle generation/check in `packages/ui-shared` and ensure migration applies cleanly to fresh local DB.
2. API behavior: verify issuance endpoint returns raw join secret once; DB stores hash only; credential expires/invalidates correctly.
3. Rust build: compile `packages/c-plane` with protobuf codegen and run tests for bootstrap validation/atomic credential consumption.
4. Go build/test: compile `packages/c-plane-agent`; run unit tests for reconnect/backoff, bootstrap request formatting, and stream handling.
5. End-to-end local: in docker compose, create cluster + join credential via ui-studio API, start agent with secret, observe cluster lifecycle transition and periodic heartbeat updates.
6. Negative tests: reused credential fails, expired credential fails, unknown cluster fails, stream disconnect triggers reconnect without re-bootstrap.
7. Security checks: confirm logs contain no raw secrets and cluster endpoints cannot access tenant-scoped APIs.

**Decisions**
- Phase 1 scope: end-to-end bootstrap MVP (control-plane + Go agent).
- Go organization: root `go.work` plus package module in `packages/c-plane-agent`.
- Transport: gRPC bidirectional streaming for runtime communication.
- Included in MVP: bootstrap credential issuance, bootstrap exchange, runtime stream heartbeat, lifecycle transition to healthy.
- Excluded from MVP: full workload scheduler integration, operator installation (Cilium/Kata), advanced mTLS PKI automation, and multi-cluster recovery orchestration.

**Further Considerations**
1. Runtime credential form: API-key-like bearer token vs asymmetric keypair signed by control plane. Recommendation: start with bearer token for MVP speed, leave proto fields extensible for future keypair/mTLS upgrade.
2. Stream ownership model: single stream per cluster identity with server-side lease/epoch to prevent split-brain reconnects. Recommendation: enforce latest-connection-wins with heartbeat timeout.
3. Migration compatibility: enum extension in Postgres can be one-way. Recommendation: append new values carefully and treat rollback as forward-fix migration.

**Upgrade and Reconnect Semantics**
1. Stable endpoint requirement: agents connect to a stable control-plane address (DNS or load balancer), never directly to a single pod IP.
2. Runtime identity continuity: post-bootstrap cluster runtime credentials are persisted in the database and remain valid across Rust API restarts and rolling deployments.
3. Reconnect behavior: if the gRPC stream drops during deployment, agents must reconnect automatically using exponential backoff with jitter and without re-running bootstrap.
4. Graceful rollout expectation: Rust API deployments should use rolling updates with overlap and graceful shutdown/drain so existing streams close cleanly and reconnect to healthy instances.
5. Compatibility contract: protobuf and auth changes must be backward compatible for at least one adjacent version window (N with N-1 agents) to avoid reconnect failures during staggered upgrades.
6. Re-bootstrap boundary: agents only re-bootstrap when runtime identity is explicitly revoked/expired, not during normal server upgrades.
7. Split-brain prevention: server enforces single active stream per cluster identity using lease/epoch semantics; latest valid connection wins and older sessions are terminated.
8. Observability requirement: emit structured events/metrics for disconnect reason, reconnect attempts, reconnect success latency, and auth rejection cause to validate rollout safety.

**Upgrade Failure Modes to Test**
1. Breaking protobuf field or service changes between deployments causing stream handshake failure.
2. Incompatible auth validation changes that reject previously valid runtime credentials.
3. Full cutover with no instance overlap causing avoidable reconnect storms.
4. Endpoint drift where agents still point to a retired address.
