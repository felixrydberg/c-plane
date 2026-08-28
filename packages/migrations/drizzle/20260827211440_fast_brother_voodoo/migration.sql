CREATE TYPE "cluster_health_status" AS ENUM('healthy', 'degraded', 'offline');--> statement-breakpoint
CREATE TYPE "cluster_ingress_endpoint_health_status" AS ENUM('healthy', 'degraded', 'unreachable');--> statement-breakpoint
CREATE TYPE "cluster_provider" AS ENUM('aws', 'gcp', 'azure', 'metal');--> statement-breakpoint
CREATE TYPE "cluster_status" AS ENUM('pending', 'bootstrapping', 'healthy', 'draining', 'offline', 'removed');--> statement-breakpoint
CREATE TYPE "foundation_bucket_status" AS ENUM('active', 'deleting');--> statement-breakpoint
CREATE TYPE "region_routing_mode" AS ENUM('active', 'draining', 'disabled');--> statement-breakpoint
CREATE TYPE "region_status" AS ENUM('active', 'inactive', 'maintenance');--> statement-breakpoint
CREATE TYPE "secret_scope" AS ENUM('platform', 'tenant');--> statement-breakpoint
CREATE TYPE "storage_status" AS ENUM('provisioning', 'ready', 'deleting', 'failed');--> statement-breakpoint
CREATE TYPE "api_key_scopes_type" AS ENUM('region:read', 'project:read', 'project:create', 'project:delete', 'project:manage', 'access-token:read', 'access-token:create', 'access-token:update', 'access-token:delete', 'bucket:read', 'bucket:create', 'bucket:delete', 'timeline:read', 'event:read', 'container:read', 'container:create', 'container:update', 'container:delete', 'database:postgres:read', 'database:postgres:create', 'database:postgres:update', 'database:postgres:delete', 'database:postgres:manage', 'registry:read', 'registry:create', 'registry:update', 'registry:delete');--> statement-breakpoint
CREATE TYPE "organization_invitation_status" AS ENUM('pending', 'accepted', 'declined', 'revoked');--> statement-breakpoint
CREATE TABLE "event" (
	"id" uuid PRIMARY KEY,
	"organization_id" uuid NOT NULL,
	"type" text NOT NULL,
	"payload" json NOT NULL,
	"system" boolean DEFAULT false NOT NULL,
	"project_id" uuid,
	"actor_id" uuid,
	"created_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "event" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "clusters" (
	"id" uuid PRIMARY KEY,
	"region_id" uuid NOT NULL,
	"slug" text NOT NULL UNIQUE,
	"name" text NOT NULL,
	"agent_id" text UNIQUE,
	"agent_endpoint" text,
	"status" "cluster_status" DEFAULT 'pending'::"cluster_status" NOT NULL,
	"provider" "cluster_provider" DEFAULT 'aws'::"cluster_provider" NOT NULL,
	"capacity_allocatable" integer DEFAULT 0 NOT NULL,
	"capacity_used" integer DEFAULT 0 NOT NULL,
	"health_status" "cluster_health_status" DEFAULT 'healthy'::"cluster_health_status" NOT NULL,
	"agent_last_seen_at" timestamp with time zone,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "clusters" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "cluster_ingress_endpoints" (
	"id" uuid PRIMARY KEY,
	"cluster_id" uuid NOT NULL,
	"address" text NOT NULL,
	"port" integer DEFAULT 443 NOT NULL,
	"enabled" boolean DEFAULT true NOT NULL,
	"health_status" "cluster_ingress_endpoint_health_status" DEFAULT 'healthy'::"cluster_ingress_endpoint_health_status" NOT NULL,
	"last_seen_at" timestamp with time zone,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "cluster_ingress_endpoints" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "cluster_join_credentials" (
	"id" uuid PRIMARY KEY,
	"cluster_id" uuid NOT NULL,
	"token_hash" text NOT NULL,
	"expires_at" timestamp with time zone NOT NULL,
	"used_at" timestamp with time zone,
	"revoked_at" timestamp with time zone,
	"revoked_reason" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "cluster_join_credentials" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "infrastructure_audit_log" (
	"id" uuid PRIMARY KEY,
	"actor_identifier" text NOT NULL,
	"source_ip" text NOT NULL,
	"action" text NOT NULL,
	"resource_type" text NOT NULL,
	"resource_id" uuid,
	"changes" jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "infrastructure_audit_log" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "bucket" (
	"id" uuid PRIMARY KEY,
	"region_id" uuid NOT NULL,
	"sse_secret_id" uuid NOT NULL,
	"status" "foundation_bucket_status" DEFAULT 'active'::"foundation_bucket_status" NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "bucket" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "bucket_grant" (
	"id" uuid PRIMARY KEY,
	"credential_id" uuid NOT NULL,
	"bucket_id" uuid NOT NULL,
	"organization_id" uuid,
	"prefix" text DEFAULT '' NOT NULL,
	"can_read" boolean DEFAULT false NOT NULL,
	"can_write" boolean DEFAULT false NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "bucket_grant_permission_check" CHECK ("can_read" or "can_write")
);
--> statement-breakpoint
ALTER TABLE "bucket_grant" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "s3_providers" (
	"id" uuid PRIMARY KEY,
	"name" text NOT NULL,
	"endpoint_url" text NOT NULL,
	"provider_region" text NOT NULL,
	"credential_secret_id" uuid NOT NULL,
	"mirror_provider_id" uuid,
	"is_active" boolean DEFAULT true NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "s3_providers_mirror_not_self_check" CHECK ("mirror_provider_id" is null or "mirror_provider_id" <> "id")
);
--> statement-breakpoint
ALTER TABLE "s3_providers" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "regions" (
	"id" uuid PRIMARY KEY,
	"slug" text NOT NULL UNIQUE,
	"display_name" text NOT NULL,
	"s3_provider_id" uuid,
	"status" "region_status" DEFAULT 'active'::"region_status" NOT NULL,
	"routing_mode" "region_routing_mode" DEFAULT 'active'::"region_routing_mode" NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "regions" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "registry_storage" (
	"id" uuid PRIMARY KEY,
	"service" text DEFAULT 'distribution' NOT NULL,
	"provider_id" uuid NOT NULL,
	"bucket_name" text NOT NULL,
	"physical_bucket_name" text NOT NULL,
	"access_key_id" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "registry_storage" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "credential" (
	"id" uuid PRIMARY KEY,
	"organization_id" uuid,
	"access_key_id" text NOT NULL,
	"secret_id" uuid NOT NULL,
	"revoked_at" timestamp with time zone,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "credential_id_organization_id_uidx" UNIQUE NULLS NOT DISTINCT("id","organization_id")
);
--> statement-breakpoint
ALTER TABLE "credential" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "secret" (
	"id" uuid PRIMARY KEY,
	"scope" "secret_scope" NOT NULL,
	"organization_id" uuid,
	"ciphertext" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "secret_id_scope_organization_uidx" UNIQUE NULLS NOT DISTINCT("id","scope","organization_id"),
	CONSTRAINT "secret_id_organization_id_uidx" UNIQUE NULLS NOT DISTINCT("id","organization_id"),
	CONSTRAINT "secret_scope_organization_check" CHECK ((
    ("scope" = 'platform' and "organization_id" is null)
    or ("scope" = 'tenant' and "organization_id" is not null)
  ))
);
--> statement-breakpoint
ALTER TABLE "secret" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "registry_maintenance" (
	"service" text PRIMARY KEY DEFAULT 'distribution',
	"gc_access_key_id" text NOT NULL UNIQUE,
	"phase" text DEFAULT 'idle' NOT NULL,
	"active_job_id" uuid,
	"started_at" timestamp with time zone,
	"finished_at" timestamp with time zone,
	"last_result" text,
	"last_error" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "registry_maintenance_phase_check" CHECK ("phase" in ('idle', 'queued', 'draining', 'collecting', 'restoring'))
);
--> statement-breakpoint
ALTER TABLE "registry_maintenance" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "worker_queue" (
	"id" uuid PRIMARY KEY,
	"organization_id" uuid,
	"queue_name" text NOT NULL,
	"job_type" text NOT NULL,
	"payload" jsonb DEFAULT '{}' NOT NULL,
	"status" text DEFAULT 'queued' NOT NULL,
	"dedupe_key" text,
	"attempts" integer DEFAULT 0 NOT NULL,
	"max_attempts" integer DEFAULT 3 NOT NULL,
	"available_at" timestamp with time zone DEFAULT now() NOT NULL,
	"locked_by" text,
	"lease_expires_at" timestamp with time zone,
	"last_error" text,
	"started_at" timestamp with time zone,
	"finished_at" timestamp with time zone,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "worker_queue_status_check" CHECK ("status" in ('queued', 'running', 'succeeded', 'failed')),
	CONSTRAINT "worker_queue_attempts_check" CHECK ("attempts" >= 0 and "max_attempts" > 0)
);
--> statement-breakpoint
ALTER TABLE "worker_queue" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "project" (
	"id" uuid PRIMARY KEY,
	"organization_id" uuid NOT NULL,
	"default_environment_id" uuid,
	"name" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "project" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "project_environment" (
	"id" uuid PRIMARY KEY,
	"project_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"name" text NOT NULL,
	"is_preview" boolean DEFAULT true NOT NULL,
	"draft_timeline" uuid NOT NULL,
	"deployed_timeline" uuid NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "project_environment" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "project_timeline" (
	"id" uuid PRIMARY KEY,
	"project_id" uuid NOT NULL,
	"environment_id" uuid,
	"organization_id" uuid NOT NULL,
	"timeline" integer NOT NULL,
	"name" text,
	"parent_timeline_id" uuid,
	"pins" jsonb DEFAULT '{}' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "project_timeline_parent_scope_uidx" UNIQUE("id","project_id","organization_id")
);
--> statement-breakpoint
ALTER TABLE "project_timeline" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "container" (
	"id" uuid PRIMARY KEY,
	"project_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"name" text NOT NULL,
	"region_id" uuid NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "container" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "container_version" (
	"id" uuid PRIMARY KEY,
	"container_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"version" integer NOT NULL,
	"image" text NOT NULL,
	"resolved_image" text NOT NULL,
	"public" boolean DEFAULT false NOT NULL,
	"replica_count" integer DEFAULT 1 NOT NULL,
	"port" integer,
	"env" jsonb,
	"env_secret_refs" jsonb,
	"resources" jsonb,
	"external_registry_id" uuid,
	"health_check" jsonb,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "container_version" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "postgres_database" (
	"id" uuid PRIMARY KEY,
	"project_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"default_branch_id" uuid,
	"name" text NOT NULL
);
--> statement-breakpoint
ALTER TABLE "postgres_database" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "postgres_database_branch" (
	"id" uuid PRIMARY KEY,
	"database_id" uuid NOT NULL,
	"branch_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"backup_retention_days" integer DEFAULT 30,
	"cpu" text,
	"ram" text,
	"high_availability" boolean DEFAULT false NOT NULL,
	"read_replicas" integer,
	"autoscaling_enabled" boolean DEFAULT false NOT NULL,
	"autoscaling_min_cpu" text,
	"autoscaling_max_cpu" text
);
--> statement-breakpoint
ALTER TABLE "postgres_database_branch" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "storage" (
	"id" uuid PRIMARY KEY,
	"project_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"region_id" uuid NOT NULL,
	"name" text NOT NULL,
	"status" "storage_status" DEFAULT 'provisioning'::"storage_status" NOT NULL,
	CONSTRAINT "storage_id_organization_id_uidx" UNIQUE("id","organization_id")
);
--> statement-breakpoint
ALTER TABLE "storage" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "storage_access_token" (
	"id" uuid PRIMARY KEY,
	"organization_id" uuid NOT NULL,
	"project_id" uuid NOT NULL,
	"name" text NOT NULL,
	"access_key_id" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"revoked_at" timestamp with time zone,
	CONSTRAINT "storage_access_token_id_organization_id_uidx" UNIQUE("id","organization_id")
);
--> statement-breakpoint
ALTER TABLE "storage_access_token" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "storage_access_token_bucket" (
	"access_token_id" uuid NOT NULL,
	"bucket_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"can_read" boolean DEFAULT false NOT NULL,
	"can_write" boolean DEFAULT false NOT NULL
);
--> statement-breakpoint
ALTER TABLE "storage_access_token_bucket" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "api_key_scopes" (
	"id" uuid PRIMARY KEY,
	"api_key_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"scope" "api_key_scopes_type" NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "api_key_scopes" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "api_keys" (
	"id" uuid PRIMARY KEY,
	"organization_id" uuid NOT NULL,
	"name" text NOT NULL,
	"key_hash" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"expires_at" integer,
	"allowed_ips" text,
	CONSTRAINT "api_keys_id_organization_id_uidx" UNIQUE("id","organization_id")
);
--> statement-breakpoint
ALTER TABLE "api_keys" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "active_organization" (
	"user_id" uuid PRIMARY KEY UNIQUE,
	"organization_id" uuid NOT NULL
);
--> statement-breakpoint
ALTER TABLE "active_organization" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "organization" (
	"id" uuid PRIMARY KEY,
	"name" text NOT NULL,
	"email" text NOT NULL UNIQUE,
	"slug" text NOT NULL UNIQUE,
	"logo" text,
	"created_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "organization" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "organization_invitation" (
	"id" uuid PRIMARY KEY,
	"organization_id" uuid NOT NULL,
	"email" text NOT NULL,
	"role" text DEFAULT 'member' NOT NULL,
	"status" "organization_invitation_status" DEFAULT 'pending'::"organization_invitation_status" NOT NULL,
	"expires_at" timestamp NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"inviter_id" uuid NOT NULL
);
--> statement-breakpoint
ALTER TABLE "organization_invitation" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "organization_member" (
	"id" uuid PRIMARY KEY,
	"organization_id" uuid NOT NULL,
	"user_id" uuid NOT NULL,
	"role" text DEFAULT 'member' NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "organization_member" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "external_registry" (
	"id" uuid PRIMARY KEY,
	"organization_id" uuid NOT NULL,
	"name" text NOT NULL,
	"host" text NOT NULL,
	"username" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "external_registry_id_organization_id_uidx" UNIQUE("id","organization_id")
);
--> statement-breakpoint
ALTER TABLE "external_registry" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "registry_access_tokens" (
	"id" uuid PRIMARY KEY,
	"organization_id" uuid NOT NULL,
	"name" text NOT NULL,
	"token_hash" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"revoked_at" timestamp with time zone,
	CONSTRAINT "registry_access_tokens_id_organization_id_uidx" UNIQUE("id","organization_id")
);
--> statement-breakpoint
ALTER TABLE "registry_access_tokens" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "registry_repositories" (
	"id" uuid PRIMARY KEY,
	"organization_id" uuid NOT NULL,
	"name" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "registry_repositories_id_organization_id_uidx" UNIQUE("id","organization_id")
);
--> statement-breakpoint
ALTER TABLE "registry_repositories" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "registry_repository_grants" (
	"id" uuid PRIMARY KEY,
	"organization_id" uuid NOT NULL,
	"repository_id" uuid NOT NULL,
	"access_token_id" uuid NOT NULL,
	"can_pull" boolean DEFAULT false NOT NULL,
	"can_push" boolean DEFAULT false NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "registry_repository_grants" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "account" (
	"id" uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid(),
	"account_id" text NOT NULL,
	"provider_id" text NOT NULL,
	"user_id" uuid NOT NULL,
	"access_token" text,
	"refresh_token" text,
	"id_token" text,
	"access_token_expires_at" timestamp,
	"refresh_token_expires_at" timestamp,
	"scope" text,
	"password" text,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp NOT NULL
);
--> statement-breakpoint
CREATE TABLE "passkey" (
	"id" uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid(),
	"name" text,
	"public_key" text NOT NULL,
	"user_id" uuid NOT NULL,
	"credential_id" text NOT NULL,
	"counter" integer NOT NULL,
	"device_type" text NOT NULL,
	"backed_up" boolean NOT NULL,
	"transports" text,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"aaguid" text
);
--> statement-breakpoint
CREATE TABLE "two_factor" (
	"id" uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid(),
	"secret" text NOT NULL,
	"backup_codes" text NOT NULL,
	"user_id" uuid NOT NULL
);
--> statement-breakpoint
CREATE TABLE "user" (
	"id" uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid(),
	"name" text NOT NULL,
	"email" text NOT NULL UNIQUE,
	"email_verified" boolean DEFAULT false NOT NULL,
	"image" text,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"role" text,
	"banned" boolean DEFAULT false,
	"ban_reason" text,
	"ban_expires" timestamp,
	"two_factor_enabled" boolean DEFAULT false
);
--> statement-breakpoint
CREATE TABLE "verification" (
	"id" uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid(),
	"identifier" text NOT NULL,
	"value" text NOT NULL,
	"expires_at" timestamp NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE INDEX "event_organization_id_idx" ON "event" ("organization_id");--> statement-breakpoint
CREATE INDEX "event_type_idx" ON "event" ("type");--> statement-breakpoint
CREATE INDEX "event_project_idx" ON "event" ("project_id","created_at");--> statement-breakpoint
CREATE INDEX "clusters_region_id_idx" ON "clusters" ("region_id");--> statement-breakpoint
CREATE INDEX "clusters_agent_id_idx" ON "clusters" ("agent_id");--> statement-breakpoint
CREATE INDEX "clusters_slug_idx" ON "clusters" ("slug");--> statement-breakpoint
CREATE INDEX "clusters_status_idx" ON "clusters" ("status");--> statement-breakpoint
CREATE INDEX "clusters_health_status_idx" ON "clusters" ("health_status");--> statement-breakpoint
CREATE INDEX "clusters_provider_idx" ON "clusters" ("provider");--> statement-breakpoint
CREATE UNIQUE INDEX "cluster_ingress_endpoints_cluster_id_address_uidx" ON "cluster_ingress_endpoints" ("cluster_id","address");--> statement-breakpoint
CREATE INDEX "cluster_ingress_endpoints_cluster_id_idx" ON "cluster_ingress_endpoints" ("cluster_id");--> statement-breakpoint
CREATE INDEX "cluster_ingress_endpoints_health_status_idx" ON "cluster_ingress_endpoints" ("health_status");--> statement-breakpoint
CREATE INDEX "cluster_ingress_endpoints_enabled_idx" ON "cluster_ingress_endpoints" ("enabled");--> statement-breakpoint
CREATE INDEX "cluster_join_credentials_cluster_id_idx" ON "cluster_join_credentials" ("cluster_id");--> statement-breakpoint
CREATE INDEX "cluster_join_credentials_expires_at_idx" ON "cluster_join_credentials" ("expires_at");--> statement-breakpoint
CREATE INDEX "cluster_join_credentials_token_hash_idx" ON "cluster_join_credentials" ("token_hash");--> statement-breakpoint
CREATE INDEX "infrastructure_audit_log_created_at_idx" ON "infrastructure_audit_log" ("created_at");--> statement-breakpoint
CREATE INDEX "infrastructure_audit_log_resource_idx" ON "infrastructure_audit_log" ("resource_type","resource_id");--> statement-breakpoint
CREATE UNIQUE INDEX "bucket_sse_secret_id_uidx" ON "bucket" ("sse_secret_id");--> statement-breakpoint
CREATE INDEX "bucket_region_id_idx" ON "bucket" ("region_id");--> statement-breakpoint
CREATE INDEX "bucket_status_idx" ON "bucket" ("status");--> statement-breakpoint
CREATE UNIQUE INDEX "bucket_grant_credential_bucket_prefix_uidx" ON "bucket_grant" ("credential_id","bucket_id","prefix");--> statement-breakpoint
CREATE INDEX "bucket_grant_credential_id_idx" ON "bucket_grant" ("credential_id");--> statement-breakpoint
CREATE INDEX "bucket_grant_bucket_id_idx" ON "bucket_grant" ("bucket_id");--> statement-breakpoint
CREATE INDEX "bucket_grant_organization_id_idx" ON "bucket_grant" ("organization_id");--> statement-breakpoint
CREATE INDEX "s3_providers_name_idx" ON "s3_providers" ("name");--> statement-breakpoint
CREATE INDEX "s3_providers_is_active_idx" ON "s3_providers" ("is_active");--> statement-breakpoint
CREATE UNIQUE INDEX "s3_providers_credential_secret_id_uidx" ON "s3_providers" ("credential_secret_id");--> statement-breakpoint
CREATE INDEX "s3_providers_mirror_provider_id_idx" ON "s3_providers" ("mirror_provider_id");--> statement-breakpoint
CREATE INDEX "regions_slug_idx" ON "regions" ("slug");--> statement-breakpoint
CREATE INDEX "regions_status_idx" ON "regions" ("status");--> statement-breakpoint
CREATE INDEX "regions_routing_mode_idx" ON "regions" ("routing_mode");--> statement-breakpoint
CREATE INDEX "regions_s3_provider_id_idx" ON "regions" ("s3_provider_id");--> statement-breakpoint
CREATE UNIQUE INDEX "registry_storage_service_uidx" ON "registry_storage" ("service");--> statement-breakpoint
CREATE UNIQUE INDEX "registry_storage_bucket_name_uidx" ON "registry_storage" ("bucket_name");--> statement-breakpoint
CREATE UNIQUE INDEX "registry_storage_physical_bucket_name_uidx" ON "registry_storage" ("physical_bucket_name");--> statement-breakpoint
CREATE UNIQUE INDEX "registry_storage_access_key_id_uidx" ON "registry_storage" ("access_key_id");--> statement-breakpoint
CREATE INDEX "registry_storage_provider_id_idx" ON "registry_storage" ("provider_id");--> statement-breakpoint
CREATE UNIQUE INDEX "credential_access_key_id_uidx" ON "credential" ("access_key_id");--> statement-breakpoint
CREATE UNIQUE INDEX "credential_secret_id_uidx" ON "credential" ("secret_id");--> statement-breakpoint
CREATE INDEX "credential_organization_id_idx" ON "credential" ("organization_id");--> statement-breakpoint
CREATE INDEX "credential_revoked_at_idx" ON "credential" ("revoked_at");--> statement-breakpoint
CREATE INDEX "secret_organization_id_idx" ON "secret" ("organization_id");--> statement-breakpoint
CREATE INDEX "secret_scope_idx" ON "secret" ("scope");--> statement-breakpoint
CREATE INDEX "registry_maintenance_active_job_idx" ON "registry_maintenance" ("active_job_id");--> statement-breakpoint
CREATE INDEX "worker_queue_organization_id_idx" ON "worker_queue" ("organization_id");--> statement-breakpoint
CREATE INDEX "worker_queue_claim_idx" ON "worker_queue" ("queue_name","status","available_at","created_at");--> statement-breakpoint
CREATE INDEX "worker_queue_lease_idx" ON "worker_queue" ("status","lease_expires_at");--> statement-breakpoint
CREATE UNIQUE INDEX "worker_queue_active_dedupe_uidx" ON "worker_queue" ("queue_name","dedupe_key") WHERE "dedupe_key" is not null and "status" in ('queued', 'running');--> statement-breakpoint
CREATE UNIQUE INDEX "project_organization_id_name_uidx" ON "project" ("organization_id","name");--> statement-breakpoint
CREATE UNIQUE INDEX "project_id_organization_id_uidx" ON "project" ("id","organization_id");--> statement-breakpoint
CREATE INDEX "project_organization_id_idx" ON "project" ("organization_id");--> statement-breakpoint
CREATE INDEX "project_default_environment_id_idx" ON "project" ("default_environment_id");--> statement-breakpoint
CREATE UNIQUE INDEX "project_environment_project_id_name_uidx" ON "project_environment" ("project_id","name");--> statement-breakpoint
CREATE UNIQUE INDEX "project_environment_id_project_id_organization_id_uidx" ON "project_environment" ("id","project_id","organization_id");--> statement-breakpoint
CREATE INDEX "project_environment_organization_id_idx" ON "project_environment" ("organization_id");--> statement-breakpoint
CREATE INDEX "project_environment_project_id_idx" ON "project_environment" ("project_id");--> statement-breakpoint
CREATE INDEX "project_timeline_id_idx" ON "project_timeline" ("id");--> statement-breakpoint
CREATE INDEX "project_timeline_environment_id_idx" ON "project_timeline" ("environment_id");--> statement-breakpoint
CREATE INDEX "project_timeline_organization_id_idx" ON "project_timeline" ("organization_id");--> statement-breakpoint
CREATE INDEX "project_timeline_project_id_idx" ON "project_timeline" ("project_id");--> statement-breakpoint
CREATE INDEX "project_timeline_parent_timeline_id_idx" ON "project_timeline" ("parent_timeline_id");--> statement-breakpoint
CREATE INDEX "container_organization_id_idx" ON "container" ("organization_id");--> statement-breakpoint
CREATE INDEX "container_project_id_idx" ON "container" ("project_id");--> statement-breakpoint
CREATE UNIQUE INDEX "container_version_container_id_version_uidx" ON "container_version" ("container_id","version");--> statement-breakpoint
CREATE INDEX "container_version_container_id_idx" ON "container_version" ("container_id");--> statement-breakpoint
CREATE INDEX "container_version_organization_id_idx" ON "container_version" ("organization_id");--> statement-breakpoint
CREATE INDEX "container_version_external_registry_id_idx" ON "container_version" ("external_registry_id");--> statement-breakpoint
CREATE INDEX "postgres_database_project_id_idx" ON "postgres_database" ("project_id");--> statement-breakpoint
CREATE INDEX "postgres_database_organization_id_idx" ON "postgres_database" ("organization_id");--> statement-breakpoint
CREATE INDEX "postgres_database_branch_database_id_idx" ON "postgres_database_branch" ("database_id");--> statement-breakpoint
CREATE INDEX "postgres_database_branch_branch_id_idx" ON "postgres_database_branch" ("branch_id");--> statement-breakpoint
CREATE INDEX "postgres_database_branch_organization_id_idx" ON "postgres_database_branch" ("organization_id");--> statement-breakpoint
CREATE UNIQUE INDEX "storage_name_idx" ON "storage" ("name");--> statement-breakpoint
CREATE INDEX "storage_project_id_idx" ON "storage" ("project_id");--> statement-breakpoint
CREATE INDEX "storage_organization_id_idx" ON "storage" ("organization_id");--> statement-breakpoint
CREATE INDEX "storage_region_id_idx" ON "storage" ("region_id");--> statement-breakpoint
CREATE UNIQUE INDEX "storage_access_token_access_key_id_uidx" ON "storage_access_token" ("access_key_id");--> statement-breakpoint
CREATE UNIQUE INDEX "storage_access_token_project_name_uidx" ON "storage_access_token" ("project_id","name") WHERE "revoked_at" is null;--> statement-breakpoint
CREATE INDEX "storage_access_token_organization_id_idx" ON "storage_access_token" ("organization_id");--> statement-breakpoint
CREATE INDEX "storage_access_token_project_id_idx" ON "storage_access_token" ("project_id");--> statement-breakpoint
CREATE UNIQUE INDEX "storage_access_token_bucket_uidx" ON "storage_access_token_bucket" ("access_token_id","bucket_id");--> statement-breakpoint
CREATE INDEX "storage_access_token_bucket_token_id_idx" ON "storage_access_token_bucket" ("access_token_id");--> statement-breakpoint
CREATE INDEX "storage_access_token_bucket_bucket_id_idx" ON "storage_access_token_bucket" ("bucket_id");--> statement-breakpoint
CREATE INDEX "api_key_scopes_api_key_id_idx" ON "api_key_scopes" ("api_key_id");--> statement-breakpoint
CREATE INDEX "api_key_scopes_scope_idx" ON "api_key_scopes" ("scope");--> statement-breakpoint
CREATE INDEX "api_key_scopes_organization_id_idx" ON "api_key_scopes" ("organization_id");--> statement-breakpoint
CREATE INDEX "api_keys_organization_id_idx" ON "api_keys" ("organization_id");--> statement-breakpoint
CREATE INDEX "api_keys_key_hash_idx" ON "api_keys" ("key_hash");--> statement-breakpoint
CREATE INDEX "active_organization_user_id_idx" ON "active_organization" ("user_id");--> statement-breakpoint
CREATE UNIQUE INDEX "organization_slug_uidx" ON "organization" ("slug");--> statement-breakpoint
CREATE INDEX "organization_id_idx" ON "organization" ("id");--> statement-breakpoint
CREATE INDEX "organization_invitation_organization_id_idx" ON "organization_invitation" ("organization_id");--> statement-breakpoint
CREATE INDEX "organization_invitation_email_idx" ON "organization_invitation" ("email");--> statement-breakpoint
CREATE UNIQUE INDEX "organization_member_user_id_organization_id_uidx" ON "organization_member" ("user_id","organization_id");--> statement-breakpoint
CREATE INDEX "organization_member_organization_id_idx" ON "organization_member" ("organization_id");--> statement-breakpoint
CREATE INDEX "organization_member_user_id_idx" ON "organization_member" ("user_id");--> statement-breakpoint
CREATE UNIQUE INDEX "external_registry_organization_name_uidx" ON "external_registry" ("organization_id","name");--> statement-breakpoint
CREATE UNIQUE INDEX "external_registry_organization_host_username_uidx" ON "external_registry" ("organization_id","host","username");--> statement-breakpoint
CREATE INDEX "external_registry_organization_id_idx" ON "external_registry" ("organization_id");--> statement-breakpoint
CREATE UNIQUE INDEX "registry_access_tokens_hash_uidx" ON "registry_access_tokens" ("token_hash");--> statement-breakpoint
CREATE UNIQUE INDEX "registry_access_tokens_organization_name_uidx" ON "registry_access_tokens" ("organization_id","name") WHERE "revoked_at" is null;--> statement-breakpoint
CREATE INDEX "registry_access_tokens_organization_id_idx" ON "registry_access_tokens" ("organization_id");--> statement-breakpoint
CREATE UNIQUE INDEX "registry_repositories_organization_name_uidx" ON "registry_repositories" ("organization_id","name");--> statement-breakpoint
CREATE INDEX "registry_repositories_organization_id_idx" ON "registry_repositories" ("organization_id");--> statement-breakpoint
CREATE UNIQUE INDEX "registry_repository_grants_token_repository_uidx" ON "registry_repository_grants" ("access_token_id","repository_id");--> statement-breakpoint
CREATE INDEX "registry_repository_grants_organization_id_idx" ON "registry_repository_grants" ("organization_id");--> statement-breakpoint
CREATE INDEX "registry_repository_grants_repository_id_idx" ON "registry_repository_grants" ("repository_id");--> statement-breakpoint
CREATE INDEX "registry_repository_grants_access_token_id_idx" ON "registry_repository_grants" ("access_token_id");--> statement-breakpoint
CREATE INDEX "account_userId_idx" ON "account" ("user_id");--> statement-breakpoint
CREATE INDEX "passkey_userId_idx" ON "passkey" ("user_id");--> statement-breakpoint
CREATE INDEX "passkey_credentialID_idx" ON "passkey" ("credential_id");--> statement-breakpoint
CREATE INDEX "twoFactor_secret_idx" ON "two_factor" ("secret");--> statement-breakpoint
CREATE INDEX "twoFactor_userId_idx" ON "two_factor" ("user_id");--> statement-breakpoint
CREATE INDEX "verification_identifier_idx" ON "verification" ("identifier");--> statement-breakpoint
ALTER TABLE "event" ADD CONSTRAINT "event_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "clusters" ADD CONSTRAINT "clusters_region_id_regions_id_fkey" FOREIGN KEY ("region_id") REFERENCES "regions"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "cluster_ingress_endpoints" ADD CONSTRAINT "cluster_ingress_endpoints_cluster_id_clusters_id_fkey" FOREIGN KEY ("cluster_id") REFERENCES "clusters"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "cluster_join_credentials" ADD CONSTRAINT "cluster_join_credentials_cluster_id_clusters_id_fkey" FOREIGN KEY ("cluster_id") REFERENCES "clusters"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "bucket" ADD CONSTRAINT "bucket_region_id_regions_id_fkey" FOREIGN KEY ("region_id") REFERENCES "regions"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "bucket" ADD CONSTRAINT "bucket_sse_secret_id_secret_id_fkey" FOREIGN KEY ("sse_secret_id") REFERENCES "secret"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "bucket_grant" ADD CONSTRAINT "bucket_grant_credential_id_credential_id_fkey" FOREIGN KEY ("credential_id","organization_id") REFERENCES "credential"("id","organization_id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "bucket_grant" ADD CONSTRAINT "bucket_grant_bucket_id_fk" FOREIGN KEY ("bucket_id") REFERENCES "bucket"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "s3_providers" ADD CONSTRAINT "s3_providers_credential_secret_id_fk" FOREIGN KEY ("credential_secret_id") REFERENCES "secret"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "s3_providers" ADD CONSTRAINT "s3_providers_mirror_provider_id_fk" FOREIGN KEY ("mirror_provider_id") REFERENCES "s3_providers"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "regions" ADD CONSTRAINT "regions_s3_provider_id_s3_providers_id_fkey" FOREIGN KEY ("s3_provider_id") REFERENCES "s3_providers"("id") ON DELETE SET NULL;--> statement-breakpoint
ALTER TABLE "registry_storage" ADD CONSTRAINT "registry_storage_provider_id_s3_providers_id_fkey" FOREIGN KEY ("provider_id") REFERENCES "s3_providers"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "credential" ADD CONSTRAINT "credential_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "credential" ADD CONSTRAINT "credential_secret_id_fk" FOREIGN KEY ("secret_id","organization_id") REFERENCES "secret"("id","organization_id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "secret" ADD CONSTRAINT "secret_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "registry_maintenance" ADD CONSTRAINT "registry_maintenance_active_job_id_worker_queue_id_fkey" FOREIGN KEY ("active_job_id") REFERENCES "worker_queue"("id") ON DELETE SET NULL;--> statement-breakpoint
ALTER TABLE "worker_queue" ADD CONSTRAINT "worker_queue_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "project" ADD CONSTRAINT "project_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "project" ADD CONSTRAINT "project_default_environment_id_project_environment_id_fkey" FOREIGN KEY ("default_environment_id") REFERENCES "project_environment"("id") ON DELETE SET NULL;--> statement-breakpoint
ALTER TABLE "project_environment" ADD CONSTRAINT "project_environment_project_id_project_id_fkey" FOREIGN KEY ("project_id") REFERENCES "project"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "project_environment" ADD CONSTRAINT "project_environment_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "project_environment" ADD CONSTRAINT "project_environment_draft_timeline_project_timeline_id_fkey" FOREIGN KEY ("draft_timeline") REFERENCES "project_timeline"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "project_environment" ADD CONSTRAINT "project_environment_deployed_timeline_project_timeline_id_fkey" FOREIGN KEY ("deployed_timeline") REFERENCES "project_timeline"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "project_timeline" ADD CONSTRAINT "project_timeline_project_id_project_id_fkey" FOREIGN KEY ("project_id") REFERENCES "project"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "project_timeline" ADD CONSTRAINT "project_timeline_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "project_timeline" ADD CONSTRAINT "project_timeline_parent_scope_fk" FOREIGN KEY ("parent_timeline_id","project_id","organization_id") REFERENCES "project_timeline"("id","project_id","organization_id");--> statement-breakpoint
ALTER TABLE "container" ADD CONSTRAINT "container_project_id_project_id_fkey" FOREIGN KEY ("project_id") REFERENCES "project"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "container" ADD CONSTRAINT "container_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "container" ADD CONSTRAINT "container_region_id_regions_id_fkey" FOREIGN KEY ("region_id") REFERENCES "regions"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "container_version" ADD CONSTRAINT "container_version_container_id_container_id_fkey" FOREIGN KEY ("container_id") REFERENCES "container"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "container_version" ADD CONSTRAINT "container_version_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "container_version" ADD CONSTRAINT "container_version_external_registry_fk" FOREIGN KEY ("external_registry_id","organization_id") REFERENCES "external_registry"("id","organization_id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "postgres_database" ADD CONSTRAINT "postgres_database_project_id_project_id_fkey" FOREIGN KEY ("project_id") REFERENCES "project"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "postgres_database" ADD CONSTRAINT "postgres_database_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "postgres_database" ADD CONSTRAINT "postgres_database_sQcvj8ruZ82p_fkey" FOREIGN KEY ("default_branch_id") REFERENCES "postgres_database_branch"("id") ON DELETE SET NULL;--> statement-breakpoint
ALTER TABLE "postgres_database_branch" ADD CONSTRAINT "postgres_database_branch_database_id_postgres_database_id_fkey" FOREIGN KEY ("database_id") REFERENCES "postgres_database"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "postgres_database_branch" ADD CONSTRAINT "postgres_database_branch_branch_id_project_environment_id_fkey" FOREIGN KEY ("branch_id") REFERENCES "project_environment"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "postgres_database_branch" ADD CONSTRAINT "postgres_database_branch_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "storage" ADD CONSTRAINT "storage_project_id_project_id_fkey" FOREIGN KEY ("project_id") REFERENCES "project"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "storage" ADD CONSTRAINT "storage_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "storage" ADD CONSTRAINT "storage_region_id_regions_id_fkey" FOREIGN KEY ("region_id") REFERENCES "regions"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "storage_access_token" ADD CONSTRAINT "storage_access_token_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "storage_access_token" ADD CONSTRAINT "storage_access_token_project_id_project_id_fkey" FOREIGN KEY ("project_id") REFERENCES "project"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "storage_access_token_bucket" ADD CONSTRAINT "storage_access_token_bucket_vSGyhrH3NafP_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "storage_access_token_bucket" ADD CONSTRAINT "storage_access_token_bucket_token_scope_fk" FOREIGN KEY ("access_token_id","organization_id") REFERENCES "storage_access_token"("id","organization_id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "storage_access_token_bucket" ADD CONSTRAINT "storage_access_token_bucket_bucket_scope_fk" FOREIGN KEY ("bucket_id","organization_id") REFERENCES "storage"("id","organization_id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "api_key_scopes" ADD CONSTRAINT "api_key_scopes_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "api_key_scopes" ADD CONSTRAINT "api_key_scopes_api_key_scope_fk" FOREIGN KEY ("api_key_id","organization_id") REFERENCES "api_keys"("id","organization_id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "api_keys" ADD CONSTRAINT "api_keys_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "active_organization" ADD CONSTRAINT "active_organization_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "active_organization" ADD CONSTRAINT "active_organization_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "organization_invitation" ADD CONSTRAINT "organization_invitation_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "organization_invitation" ADD CONSTRAINT "organization_invitation_inviter_id_user_id_fkey" FOREIGN KEY ("inviter_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "organization_member" ADD CONSTRAINT "organization_member_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "organization_member" ADD CONSTRAINT "organization_member_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "external_registry" ADD CONSTRAINT "external_registry_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "registry_access_tokens" ADD CONSTRAINT "registry_access_tokens_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "registry_repositories" ADD CONSTRAINT "registry_repositories_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "registry_repository_grants" ADD CONSTRAINT "registry_repository_grants_organization_id_organization_id_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "registry_repository_grants" ADD CONSTRAINT "registry_repository_grants_repository_scope_fk" FOREIGN KEY ("repository_id","organization_id") REFERENCES "registry_repositories"("id","organization_id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "registry_repository_grants" ADD CONSTRAINT "registry_repository_grants_token_scope_fk" FOREIGN KEY ("access_token_id","organization_id") REFERENCES "registry_access_tokens"("id","organization_id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "account" ADD CONSTRAINT "account_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "passkey" ADD CONSTRAINT "passkey_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "two_factor" ADD CONSTRAINT "two_factor_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
CREATE POLICY "event_org_rls" ON "event" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("event"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("event"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "infrastructure_audit_log_reader" ON "infrastructure_audit_log" AS PERMISSIVE FOR SELECT TO "app_audit_reader" USING (true);--> statement-breakpoint
CREATE POLICY "bucket_tenant_select_rls" ON "bucket" AS PERMISSIVE FOR SELECT TO "app_tenant" USING (true);--> statement-breakpoint
CREATE POLICY "bucket_grant_platform_select_rls" ON "bucket_grant" AS PERMISSIVE FOR SELECT TO "app_tenant" USING ("bucket_grant"."organization_id" is null);--> statement-breakpoint
CREATE POLICY "bucket_grant_tenant_rls" ON "bucket_grant" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("bucket_grant"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("bucket_grant"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "s3_providers_tenant_select_rls" ON "s3_providers" AS PERMISSIVE FOR SELECT TO "app_tenant" USING (true);--> statement-breakpoint
CREATE POLICY "regions_tenant_select_rls" ON "regions" AS PERMISSIVE FOR SELECT TO "app_tenant" USING (true);--> statement-breakpoint
CREATE POLICY "registry_storage_tenant_select_rls" ON "registry_storage" AS PERMISSIVE FOR SELECT TO "app_tenant" USING (true);--> statement-breakpoint
CREATE POLICY "credential_platform_select_rls" ON "credential" AS PERMISSIVE FOR SELECT TO "app_tenant" USING ("credential"."organization_id" is null);--> statement-breakpoint
CREATE POLICY "credential_tenant_rls" ON "credential" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("credential"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("credential"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "secret_platform_select_rls" ON "secret" AS PERMISSIVE FOR SELECT TO "app_tenant" USING ("secret"."scope" = 'platform');--> statement-breakpoint
CREATE POLICY "secret_tenant_rls" ON "secret" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("secret"."scope" = 'tenant' and "secret"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("secret"."scope" = 'tenant' and "secret"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "registry_maintenance_tenant_select_rls" ON "registry_maintenance" AS PERMISSIVE FOR SELECT TO "app_tenant" USING (true);--> statement-breakpoint
CREATE POLICY "project_tenant_rls" ON "project" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("project"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("project"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "project_environment_tenant_rls" ON "project_environment" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("project_environment"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("project_environment"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "project_timeline_tenant_rls" ON "project_timeline" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("project_timeline"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("project_timeline"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "container_tenant_rls" ON "container" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("container"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("container"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "container_version_tenant_rls" ON "container_version" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("container_version"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("container_version"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "postgres_database_tenant_rls" ON "postgres_database" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("postgres_database"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("postgres_database"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "postgres_database_branch_tenant_rls" ON "postgres_database_branch" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("postgres_database_branch"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("postgres_database_branch"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "storage_tenant_rls" ON "storage" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("storage"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("storage"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "storage_access_token_tenant_rls" ON "storage_access_token" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("storage_access_token"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("storage_access_token"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "storage_access_token_bucket_tenant_rls" ON "storage_access_token_bucket" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("storage_access_token_bucket"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("storage_access_token_bucket"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "api_key_scopes_tenant_rls" ON "api_key_scopes" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("api_key_scopes"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("api_key_scopes"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "api_keys_tenant_rls" ON "api_keys" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("api_keys"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("api_keys"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "active_organization_tenant_rls" ON "active_organization" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("active_organization"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("active_organization"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "organization_tenant_rls_select" ON "organization" AS PERMISSIVE FOR SELECT TO "app_tenant" USING ("organization"."id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "organization_tenant_rls_update" ON "organization" AS PERMISSIVE FOR UPDATE TO "app_tenant" USING ("organization"."id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("organization"."id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "organization_tenant_rls_delete" ON "organization" AS PERMISSIVE FOR DELETE TO "app_tenant" USING ("organization"."id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "organization_tenant_rls_insert" ON "organization" AS PERMISSIVE FOR INSERT TO "app_tenant" WITH CHECK (true);--> statement-breakpoint
CREATE POLICY "organization_invitation_tenant_rls" ON "organization_invitation" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("organization_invitation"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("organization_invitation"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "organization_member_tenant_rls" ON "organization_member" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("organization_member"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("organization_member"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "external_registry_tenant_rls" ON "external_registry" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("external_registry"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("external_registry"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "registry_access_tokens_tenant_rls" ON "registry_access_tokens" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("registry_access_tokens"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("registry_access_tokens"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "registry_repositories_tenant_rls" ON "registry_repositories" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("registry_repositories"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("registry_repositories"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "registry_repository_grants_tenant_rls" ON "registry_repository_grants" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("registry_repository_grants"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("registry_repository_grants"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));