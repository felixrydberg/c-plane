CREATE TYPE "public"."cluster_health_status" AS ENUM('healthy', 'degraded', 'offline');--> statement-breakpoint
CREATE TYPE "public"."cluster_ingress_endpoint_health_status" AS ENUM('healthy', 'degraded', 'unreachable');--> statement-breakpoint
CREATE TYPE "public"."cluster_provider" AS ENUM('aws', 'gcp', 'azure', 'metal');--> statement-breakpoint
CREATE TYPE "public"."cluster_status" AS ENUM('pending', 'bootstrapping', 'healthy', 'draining', 'offline', 'removed');--> statement-breakpoint
CREATE TYPE "public"."s3_provider_type" AS ENUM('aws_s3', 'cloudflare_r2');--> statement-breakpoint
CREATE TYPE "public"."region_routing_mode" AS ENUM('active', 'draining', 'disabled');--> statement-breakpoint
CREATE TYPE "public"."region_status" AS ENUM('active', 'inactive', 'maintenance');--> statement-breakpoint
CREATE TYPE "public"."bucket_status" AS ENUM('provisioning', 'ready', 'deleting', 'failed');--> statement-breakpoint
CREATE TYPE "public"."api_key_scopes_type" AS ENUM('read:sessions', 'write:sessions');--> statement-breakpoint
CREATE TYPE "public"."organization_invitation_status" AS ENUM('pending', 'accepted', 'declined', 'revoked');--> statement-breakpoint
CREATE TABLE "event" (
	"id" uuid PRIMARY KEY NOT NULL,
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
	"id" uuid PRIMARY KEY NOT NULL,
	"region_id" uuid NOT NULL,
	"slug" text NOT NULL,
	"name" text NOT NULL,
	"agent_id" text,
	"agent_endpoint" text,
	"status" "cluster_status" DEFAULT 'pending' NOT NULL,
	"provider" "cluster_provider" DEFAULT 'aws' NOT NULL,
	"capacity_allocatable" integer DEFAULT 0 NOT NULL,
	"capacity_used" integer DEFAULT 0 NOT NULL,
	"health_status" "cluster_health_status" DEFAULT 'healthy' NOT NULL,
	"agent_last_seen_at" timestamp with time zone,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "clusters_slug_unique" UNIQUE("slug"),
	CONSTRAINT "clusters_agent_id_unique" UNIQUE("agent_id")
);
--> statement-breakpoint
ALTER TABLE "clusters" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "cluster_ingress_endpoints" (
	"id" uuid PRIMARY KEY NOT NULL,
	"cluster_id" uuid NOT NULL,
	"address" text NOT NULL,
	"port" integer DEFAULT 443 NOT NULL,
	"enabled" boolean DEFAULT true NOT NULL,
	"health_status" "cluster_ingress_endpoint_health_status" DEFAULT 'healthy' NOT NULL,
	"last_seen_at" timestamp with time zone,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "cluster_ingress_endpoints" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "cluster_join_credentials" (
	"id" uuid PRIMARY KEY NOT NULL,
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
	"id" uuid PRIMARY KEY NOT NULL,
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
CREATE TABLE "s3_providers" (
	"id" uuid PRIMARY KEY NOT NULL,
	"provider_type" "s3_provider_type" NOT NULL,
	"endpoint_url" text NOT NULL,
	"provider_region" text,
	"is_active" boolean DEFAULT true NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "s3_providers" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "regions" (
	"id" uuid PRIMARY KEY NOT NULL,
	"slug" text NOT NULL,
	"display_name" text NOT NULL,
	"s3_provider_id" uuid,
	"status" "region_status" DEFAULT 'active' NOT NULL,
	"routing_mode" "region_routing_mode" DEFAULT 'active' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "regions_slug_unique" UNIQUE("slug")
);
--> statement-breakpoint
ALTER TABLE "regions" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "project" (
	"id" uuid PRIMARY KEY NOT NULL,
	"organization_id" uuid NOT NULL,
	"default_branch_id" uuid,
	"name" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "project" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "project_branch" (
	"id" uuid PRIMARY KEY NOT NULL,
	"project_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"name" text NOT NULL,
	"timeline" uuid NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "project_branch" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "project_timeline" (
	"id" uuid PRIMARY KEY NOT NULL,
	"project_id" uuid NOT NULL,
	"branch_id" uuid,
	"organization_id" uuid NOT NULL,
	"timeline" integer NOT NULL,
	"name" text,
	"parent_timeline_id" uuid,
	"pins" jsonb DEFAULT '{}'::jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "project_timeline_parent_scope_uidx" UNIQUE("id","project_id","organization_id")
);
--> statement-breakpoint
ALTER TABLE "project_timeline" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "project_container" (
	"id" uuid PRIMARY KEY NOT NULL,
	"project_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"name" text NOT NULL,
	"region_id" uuid NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "project_container" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "project_container_version" (
	"id" uuid PRIMARY KEY NOT NULL,
	"container_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"version" integer NOT NULL,
	"image" text NOT NULL,
	"public" boolean DEFAULT false NOT NULL,
	"replica_count" integer DEFAULT 1 NOT NULL,
	"port" integer,
	"env" jsonb,
	"env_secret_refs" jsonb,
	"resources" jsonb,
	"pull_secret_id" uuid,
	"health_check" jsonb,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "project_container_version" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "serverless_postgres_database" (
	"id" uuid PRIMARY KEY NOT NULL,
	"project_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"default_branch_id" uuid,
	"name" text NOT NULL,
	"autoscaling_enabled" boolean DEFAULT false NOT NULL,
	"autoscaling_min_cpu" text,
	"autoscaling_max_cpu" text
);
--> statement-breakpoint
ALTER TABLE "serverless_postgres_database" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "serverless_postgres_database_branch" (
	"id" uuid PRIMARY KEY NOT NULL,
	"database_id" uuid NOT NULL,
	"branch_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL
);
--> statement-breakpoint
ALTER TABLE "serverless_postgres_database_branch" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "stateful_postgres_database" (
	"id" uuid PRIMARY KEY NOT NULL,
	"project_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"default_branch_id" uuid,
	"name" text NOT NULL,
	"cpu" text,
	"ram" text,
	"high_availability" boolean DEFAULT false NOT NULL,
	"read_replicas" integer,
	"autoscaling_enabled" boolean DEFAULT false NOT NULL,
	"autoscaling_min_cpu" text,
	"autoscaling_max_cpu" text
);
--> statement-breakpoint
ALTER TABLE "stateful_postgres_database" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "stateful_postgres_database_branch" (
	"id" uuid PRIMARY KEY NOT NULL,
	"database_id" uuid NOT NULL,
	"branch_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL
);
--> statement-breakpoint
ALTER TABLE "stateful_postgres_database_branch" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "bucket" (
	"id" uuid PRIMARY KEY NOT NULL,
	"project_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"region" uuid NOT NULL,
	"name" text NOT NULL,
	"is_public" boolean DEFAULT false NOT NULL,
	"status" "bucket_status" DEFAULT 'provisioning' NOT NULL
);
--> statement-breakpoint
ALTER TABLE "bucket" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "storage_access_token" (
	"id" uuid PRIMARY KEY NOT NULL,
	"organization_id" uuid NOT NULL,
	"project_id" uuid NOT NULL,
	"name" text NOT NULL,
	"access_key_id" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"revoked_at" timestamp with time zone
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
	"id" uuid PRIMARY KEY NOT NULL,
	"api_key_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"scope" "api_key_scopes_type" NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "api_key_scopes" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "api_keys" (
	"id" uuid PRIMARY KEY NOT NULL,
	"organization_id" uuid NOT NULL,
	"name" text NOT NULL,
	"key_hash" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"expires_at" integer,
	"allowed_ips" text
);
--> statement-breakpoint
ALTER TABLE "api_keys" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "active_organization" (
	"user_id" uuid PRIMARY KEY NOT NULL,
	"organization_id" uuid NOT NULL,
	CONSTRAINT "active_organization_user_id_unique" UNIQUE("user_id")
);
--> statement-breakpoint
ALTER TABLE "active_organization" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "organization" (
	"id" uuid PRIMARY KEY NOT NULL,
	"name" text NOT NULL,
	"email" text NOT NULL,
	"slug" text NOT NULL,
	"logo" text,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"polar_customer_id" uuid NOT NULL,
	CONSTRAINT "organization_email_unique" UNIQUE("email"),
	CONSTRAINT "organization_slug_unique" UNIQUE("slug"),
	CONSTRAINT "organization_polar_customer_id_unique" UNIQUE("polar_customer_id")
);
--> statement-breakpoint
ALTER TABLE "organization" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "organization_invitation" (
	"id" uuid PRIMARY KEY NOT NULL,
	"organization_id" uuid NOT NULL,
	"email" text NOT NULL,
	"role" text DEFAULT 'member' NOT NULL,
	"status" "organization_invitation_status" DEFAULT 'pending' NOT NULL,
	"expires_at" timestamp NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"inviter_id" uuid NOT NULL
);
--> statement-breakpoint
ALTER TABLE "organization_invitation" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "organization_member" (
	"id" uuid PRIMARY KEY NOT NULL,
	"organization_id" uuid NOT NULL,
	"user_id" uuid NOT NULL,
	"role" text DEFAULT 'member' NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "organization_member" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "account" (
	"id" uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid() NOT NULL,
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
CREATE TABLE "two_factor" (
	"id" uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid() NOT NULL,
	"secret" text NOT NULL,
	"backup_codes" text NOT NULL,
	"user_id" uuid NOT NULL
);
--> statement-breakpoint
CREATE TABLE "user" (
	"id" uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid() NOT NULL,
	"name" text NOT NULL,
	"email" text NOT NULL,
	"email_verified" boolean DEFAULT false NOT NULL,
	"image" text,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"role" text,
	"banned" boolean DEFAULT false,
	"ban_reason" text,
	"ban_expires" timestamp,
	"two_factor_enabled" boolean DEFAULT false,
	CONSTRAINT "user_email_unique" UNIQUE("email")
);
--> statement-breakpoint
CREATE TABLE "verification" (
	"id" uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid() NOT NULL,
	"identifier" text NOT NULL,
	"value" text NOT NULL,
	"expires_at" timestamp NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "event" ADD CONSTRAINT "event_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "clusters" ADD CONSTRAINT "clusters_region_id_regions_id_fk" FOREIGN KEY ("region_id") REFERENCES "public"."regions"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "cluster_ingress_endpoints" ADD CONSTRAINT "cluster_ingress_endpoints_cluster_id_clusters_id_fk" FOREIGN KEY ("cluster_id") REFERENCES "public"."clusters"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "cluster_join_credentials" ADD CONSTRAINT "cluster_join_credentials_cluster_id_clusters_id_fk" FOREIGN KEY ("cluster_id") REFERENCES "public"."clusters"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "regions" ADD CONSTRAINT "regions_s3_provider_id_s3_providers_id_fk" FOREIGN KEY ("s3_provider_id") REFERENCES "public"."s3_providers"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project" ADD CONSTRAINT "project_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project" ADD CONSTRAINT "project_default_branch_id_project_branch_id_fk" FOREIGN KEY ("default_branch_id") REFERENCES "public"."project_branch"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_branch" ADD CONSTRAINT "project_branch_project_id_project_id_fk" FOREIGN KEY ("project_id") REFERENCES "public"."project"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_branch" ADD CONSTRAINT "project_branch_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_branch" ADD CONSTRAINT "project_branch_timeline_project_timeline_id_fk" FOREIGN KEY ("timeline") REFERENCES "public"."project_timeline"("id") ON DELETE restrict ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_timeline" ADD CONSTRAINT "project_timeline_project_id_project_id_fk" FOREIGN KEY ("project_id") REFERENCES "public"."project"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_timeline" ADD CONSTRAINT "project_timeline_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_timeline" ADD CONSTRAINT "project_timeline_parent_timeline_id_project_timeline_id_fk" FOREIGN KEY ("parent_timeline_id") REFERENCES "public"."project_timeline"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_timeline" ADD CONSTRAINT "project_timeline_parent_scope_fk" FOREIGN KEY ("parent_timeline_id","project_id","organization_id") REFERENCES "public"."project_timeline"("id","project_id","organization_id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_container" ADD CONSTRAINT "project_container_project_id_project_id_fk" FOREIGN KEY ("project_id") REFERENCES "public"."project"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_container" ADD CONSTRAINT "project_container_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_container" ADD CONSTRAINT "project_container_region_id_regions_id_fk" FOREIGN KEY ("region_id") REFERENCES "public"."regions"("id") ON DELETE restrict ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_container_version" ADD CONSTRAINT "project_container_version_container_id_project_container_id_fk" FOREIGN KEY ("container_id") REFERENCES "public"."project_container"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_container_version" ADD CONSTRAINT "project_container_version_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "serverless_postgres_database" ADD CONSTRAINT "serverless_postgres_database_project_id_project_id_fk" FOREIGN KEY ("project_id") REFERENCES "public"."project"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "serverless_postgres_database" ADD CONSTRAINT "serverless_postgres_database_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "serverless_postgres_database" ADD CONSTRAINT "serverless_postgres_database_default_branch_id_serverless_postgres_database_branch_id_fk" FOREIGN KEY ("default_branch_id") REFERENCES "public"."serverless_postgres_database_branch"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "serverless_postgres_database_branch" ADD CONSTRAINT "serverless_postgres_database_branch_database_id_serverless_postgres_database_id_fk" FOREIGN KEY ("database_id") REFERENCES "public"."serverless_postgres_database"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "serverless_postgres_database_branch" ADD CONSTRAINT "serverless_postgres_database_branch_branch_id_project_branch_id_fk" FOREIGN KEY ("branch_id") REFERENCES "public"."project_branch"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "serverless_postgres_database_branch" ADD CONSTRAINT "serverless_postgres_database_branch_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "stateful_postgres_database" ADD CONSTRAINT "stateful_postgres_database_project_id_project_id_fk" FOREIGN KEY ("project_id") REFERENCES "public"."project"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "stateful_postgres_database" ADD CONSTRAINT "stateful_postgres_database_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "stateful_postgres_database" ADD CONSTRAINT "stateful_postgres_database_default_branch_id_stateful_postgres_database_branch_id_fk" FOREIGN KEY ("default_branch_id") REFERENCES "public"."stateful_postgres_database_branch"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "stateful_postgres_database_branch" ADD CONSTRAINT "stateful_postgres_database_branch_database_id_stateful_postgres_database_id_fk" FOREIGN KEY ("database_id") REFERENCES "public"."stateful_postgres_database"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "stateful_postgres_database_branch" ADD CONSTRAINT "stateful_postgres_database_branch_branch_id_project_branch_id_fk" FOREIGN KEY ("branch_id") REFERENCES "public"."project_branch"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "stateful_postgres_database_branch" ADD CONSTRAINT "stateful_postgres_database_branch_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "bucket" ADD CONSTRAINT "bucket_project_id_project_id_fk" FOREIGN KEY ("project_id") REFERENCES "public"."project"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "bucket" ADD CONSTRAINT "bucket_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "bucket" ADD CONSTRAINT "bucket_region_regions_id_fk" FOREIGN KEY ("region") REFERENCES "public"."regions"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "storage_access_token" ADD CONSTRAINT "storage_access_token_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "storage_access_token" ADD CONSTRAINT "storage_access_token_project_id_project_id_fk" FOREIGN KEY ("project_id") REFERENCES "public"."project"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "storage_access_token_bucket" ADD CONSTRAINT "storage_access_token_bucket_access_token_id_storage_access_token_id_fk" FOREIGN KEY ("access_token_id") REFERENCES "public"."storage_access_token"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "storage_access_token_bucket" ADD CONSTRAINT "storage_access_token_bucket_bucket_id_bucket_id_fk" FOREIGN KEY ("bucket_id") REFERENCES "public"."bucket"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "storage_access_token_bucket" ADD CONSTRAINT "storage_access_token_bucket_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "api_key_scopes" ADD CONSTRAINT "api_key_scopes_api_key_id_api_keys_id_fk" FOREIGN KEY ("api_key_id") REFERENCES "public"."api_keys"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "api_key_scopes" ADD CONSTRAINT "api_key_scopes_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "api_keys" ADD CONSTRAINT "api_keys_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "active_organization" ADD CONSTRAINT "active_organization_user_id_user_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."user"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "active_organization" ADD CONSTRAINT "active_organization_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "organization_invitation" ADD CONSTRAINT "organization_invitation_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "organization_invitation" ADD CONSTRAINT "organization_invitation_inviter_id_user_id_fk" FOREIGN KEY ("inviter_id") REFERENCES "public"."user"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "organization_member" ADD CONSTRAINT "organization_member_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "organization_member" ADD CONSTRAINT "organization_member_user_id_user_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."user"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "account" ADD CONSTRAINT "account_user_id_user_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."user"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "two_factor" ADD CONSTRAINT "two_factor_user_id_user_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."user"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "event_organization_id_idx" ON "event" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "event_type_idx" ON "event" USING btree ("type");--> statement-breakpoint
CREATE INDEX "event_project_idx" ON "event" USING btree ("project_id","created_at");--> statement-breakpoint
CREATE INDEX "clusters_region_id_idx" ON "clusters" USING btree ("region_id");--> statement-breakpoint
CREATE INDEX "clusters_agent_id_idx" ON "clusters" USING btree ("agent_id");--> statement-breakpoint
CREATE INDEX "clusters_slug_idx" ON "clusters" USING btree ("slug");--> statement-breakpoint
CREATE INDEX "clusters_status_idx" ON "clusters" USING btree ("status");--> statement-breakpoint
CREATE INDEX "clusters_health_status_idx" ON "clusters" USING btree ("health_status");--> statement-breakpoint
CREATE INDEX "clusters_provider_idx" ON "clusters" USING btree ("provider");--> statement-breakpoint
CREATE UNIQUE INDEX "cluster_ingress_endpoints_cluster_id_address_uidx" ON "cluster_ingress_endpoints" USING btree ("cluster_id","address");--> statement-breakpoint
CREATE INDEX "cluster_ingress_endpoints_cluster_id_idx" ON "cluster_ingress_endpoints" USING btree ("cluster_id");--> statement-breakpoint
CREATE INDEX "cluster_ingress_endpoints_health_status_idx" ON "cluster_ingress_endpoints" USING btree ("health_status");--> statement-breakpoint
CREATE INDEX "cluster_ingress_endpoints_enabled_idx" ON "cluster_ingress_endpoints" USING btree ("enabled");--> statement-breakpoint
CREATE INDEX "cluster_join_credentials_cluster_id_idx" ON "cluster_join_credentials" USING btree ("cluster_id");--> statement-breakpoint
CREATE INDEX "cluster_join_credentials_expires_at_idx" ON "cluster_join_credentials" USING btree ("expires_at");--> statement-breakpoint
CREATE INDEX "cluster_join_credentials_token_hash_idx" ON "cluster_join_credentials" USING btree ("token_hash");--> statement-breakpoint
CREATE INDEX "infrastructure_audit_log_created_at_idx" ON "infrastructure_audit_log" USING btree ("created_at");--> statement-breakpoint
CREATE INDEX "infrastructure_audit_log_resource_idx" ON "infrastructure_audit_log" USING btree ("resource_type","resource_id");--> statement-breakpoint
CREATE INDEX "s3_providers_provider_type_idx" ON "s3_providers" USING btree ("provider_type");--> statement-breakpoint
CREATE INDEX "s3_providers_is_active_idx" ON "s3_providers" USING btree ("is_active");--> statement-breakpoint
CREATE INDEX "regions_slug_idx" ON "regions" USING btree ("slug");--> statement-breakpoint
CREATE INDEX "regions_status_idx" ON "regions" USING btree ("status");--> statement-breakpoint
CREATE INDEX "regions_routing_mode_idx" ON "regions" USING btree ("routing_mode");--> statement-breakpoint
CREATE INDEX "regions_s3_provider_id_idx" ON "regions" USING btree ("s3_provider_id");--> statement-breakpoint
CREATE UNIQUE INDEX "project_organization_id_name_uidx" ON "project" USING btree ("organization_id","name");--> statement-breakpoint
CREATE UNIQUE INDEX "project_id_organization_id_uidx" ON "project" USING btree ("id","organization_id");--> statement-breakpoint
CREATE INDEX "project_organization_id_idx" ON "project" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "project_default_branch_id_idx" ON "project" USING btree ("default_branch_id");--> statement-breakpoint
CREATE UNIQUE INDEX "project_branch_project_id_name_uidx" ON "project_branch" USING btree ("project_id","name");--> statement-breakpoint
CREATE UNIQUE INDEX "project_branch_id_project_id_organization_id_uidx" ON "project_branch" USING btree ("id","project_id","organization_id");--> statement-breakpoint
CREATE INDEX "project_branch_organization_id_idx" ON "project_branch" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "project_branch_project_id_idx" ON "project_branch" USING btree ("project_id");--> statement-breakpoint
CREATE INDEX "project_timeline_id_idx" ON "project_timeline" USING btree ("id");--> statement-breakpoint
CREATE INDEX "project_timeline_branch_id_idx" ON "project_timeline" USING btree ("branch_id");--> statement-breakpoint
CREATE INDEX "project_timeline_organization_id_idx" ON "project_timeline" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "project_timeline_project_id_idx" ON "project_timeline" USING btree ("project_id");--> statement-breakpoint
CREATE INDEX "project_timeline_parent_timeline_id_idx" ON "project_timeline" USING btree ("parent_timeline_id");--> statement-breakpoint
CREATE INDEX "project_container_organization_id_idx" ON "project_container" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "project_container_project_id_idx" ON "project_container" USING btree ("project_id");--> statement-breakpoint
CREATE UNIQUE INDEX "project_container_version_container_id_version_uidx" ON "project_container_version" USING btree ("container_id","version");--> statement-breakpoint
CREATE INDEX "project_container_version_container_id_idx" ON "project_container_version" USING btree ("container_id");--> statement-breakpoint
CREATE INDEX "project_container_version_organization_id_idx" ON "project_container_version" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "serverless_postgres_database_project_id_idx" ON "serverless_postgres_database" USING btree ("project_id");--> statement-breakpoint
CREATE INDEX "serverless_postgres_database_organization_id_idx" ON "serverless_postgres_database" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "serverless_postgres_database_branch_database_id_idx" ON "serverless_postgres_database_branch" USING btree ("database_id");--> statement-breakpoint
CREATE INDEX "serverless_postgres_database_branch_branch_id_idx" ON "serverless_postgres_database_branch" USING btree ("branch_id");--> statement-breakpoint
CREATE INDEX "serverless_postgres_database_branch_organization_id_idx" ON "serverless_postgres_database_branch" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "stateful_postgres_database_project_id_idx" ON "stateful_postgres_database" USING btree ("project_id");--> statement-breakpoint
CREATE INDEX "stateful_postgres_database_organization_id_idx" ON "stateful_postgres_database" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "stateful_postgres_database_branch_database_id_idx" ON "stateful_postgres_database_branch" USING btree ("database_id");--> statement-breakpoint
CREATE INDEX "stateful_postgres_database_branch_branch_id_idx" ON "stateful_postgres_database_branch" USING btree ("branch_id");--> statement-breakpoint
CREATE INDEX "stateful_postgres_database_branch_organization_id_idx" ON "stateful_postgres_database_branch" USING btree ("organization_id");--> statement-breakpoint
CREATE UNIQUE INDEX "bucket_name_idx" ON "bucket" USING btree ("name");--> statement-breakpoint
CREATE INDEX "bucket_project_id_idx" ON "bucket" USING btree ("project_id");--> statement-breakpoint
CREATE INDEX "bucket_organization_id_idx" ON "bucket" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "bucket_region_idx" ON "bucket" USING btree ("region");--> statement-breakpoint
CREATE UNIQUE INDEX "storage_access_token_access_key_id_uidx" ON "storage_access_token" USING btree ("access_key_id");--> statement-breakpoint
CREATE UNIQUE INDEX "storage_access_token_project_name_uidx" ON "storage_access_token" USING btree ("project_id","name") WHERE "storage_access_token"."revoked_at" is null;--> statement-breakpoint
CREATE INDEX "storage_access_token_organization_id_idx" ON "storage_access_token" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "storage_access_token_project_id_idx" ON "storage_access_token" USING btree ("project_id");--> statement-breakpoint
CREATE UNIQUE INDEX "storage_access_token_bucket_uidx" ON "storage_access_token_bucket" USING btree ("access_token_id","bucket_id");--> statement-breakpoint
CREATE INDEX "storage_access_token_bucket_token_id_idx" ON "storage_access_token_bucket" USING btree ("access_token_id");--> statement-breakpoint
CREATE INDEX "storage_access_token_bucket_bucket_id_idx" ON "storage_access_token_bucket" USING btree ("bucket_id");--> statement-breakpoint
CREATE INDEX "api_key_scopes_api_key_id_idx" ON "api_key_scopes" USING btree ("api_key_id");--> statement-breakpoint
CREATE INDEX "api_key_scopes_scope_idx" ON "api_key_scopes" USING btree ("scope");--> statement-breakpoint
CREATE INDEX "api_key_scopes_organization_id_idx" ON "api_key_scopes" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "api_keys_organization_id_idx" ON "api_keys" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "api_keys_key_hash_idx" ON "api_keys" USING btree ("key_hash");--> statement-breakpoint
CREATE INDEX "active_organization_user_id_idx" ON "active_organization" USING btree ("user_id");--> statement-breakpoint
CREATE UNIQUE INDEX "organization_slug_uidx" ON "organization" USING btree ("slug");--> statement-breakpoint
CREATE INDEX "organization_polar_customer_id_idx" ON "organization" USING btree ("polar_customer_id");--> statement-breakpoint
CREATE INDEX "organization_id_idx" ON "organization" USING btree ("id");--> statement-breakpoint
CREATE INDEX "organization_invitation_organization_id_idx" ON "organization_invitation" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "organization_invitation_email_idx" ON "organization_invitation" USING btree ("email");--> statement-breakpoint
CREATE UNIQUE INDEX "organization_member_user_id_organization_id_uidx" ON "organization_member" USING btree ("user_id","organization_id");--> statement-breakpoint
CREATE INDEX "organization_member_organization_id_idx" ON "organization_member" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "organization_member_user_id_idx" ON "organization_member" USING btree ("user_id");--> statement-breakpoint
CREATE INDEX "account_userId_idx" ON "account" USING btree ("user_id");--> statement-breakpoint
CREATE INDEX "twoFactor_secret_idx" ON "two_factor" USING btree ("secret");--> statement-breakpoint
CREATE INDEX "twoFactor_userId_idx" ON "two_factor" USING btree ("user_id");--> statement-breakpoint
CREATE INDEX "verification_identifier_idx" ON "verification" USING btree ("identifier");--> statement-breakpoint
CREATE POLICY "event_org_rls" ON "event" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("event"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("event"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "infrastructure_audit_log_reader" ON "infrastructure_audit_log" AS PERMISSIVE FOR SELECT TO "app_audit_reader" USING (true);--> statement-breakpoint
CREATE POLICY "regions_tenant_select_rls" ON "regions" AS PERMISSIVE FOR SELECT TO "app_tenant" USING (true);--> statement-breakpoint
CREATE POLICY "project_tenant_rls" ON "project" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("project"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("project"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "project_branch_tenant_rls" ON "project_branch" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("project_branch"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("project_branch"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "project_timeline_tenant_rls" ON "project_timeline" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("project_timeline"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("project_timeline"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "project_container_tenant_rls" ON "project_container" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("project_container"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("project_container"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "project_container_version_tenant_rls" ON "project_container_version" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("project_container_version"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("project_container_version"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "serverless_postgres_database_tenant_rls" ON "serverless_postgres_database" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("serverless_postgres_database"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("serverless_postgres_database"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "serverless_postgres_database_branch_tenant_rls" ON "serverless_postgres_database_branch" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("serverless_postgres_database_branch"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("serverless_postgres_database_branch"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "stateful_postgres_database_tenant_rls" ON "stateful_postgres_database" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("stateful_postgres_database"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("stateful_postgres_database"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "stateful_postgres_database_branch_tenant_rls" ON "stateful_postgres_database_branch" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("stateful_postgres_database_branch"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("stateful_postgres_database_branch"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "bucket_tenant_rls" ON "bucket" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("bucket"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("bucket"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "storage_access_token_tenant_rls" ON "storage_access_token" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("storage_access_token"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("storage_access_token"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "storage_access_token_bucket_tenant_rls" ON "storage_access_token_bucket" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("storage_access_token_bucket"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("storage_access_token_bucket"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "api_key_scopes_tenant_rls" ON "api_key_scopes" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("api_key_scopes"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("api_key_scopes"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "api_keys_tenant_rls" ON "api_keys" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("api_keys"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("api_keys"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "active_organization_tenant_rls" ON "active_organization" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("active_organization"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("active_organization"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "organization_tenant_rls_select" ON "organization" AS PERMISSIVE FOR SELECT TO "app_tenant" USING ("organization"."id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "organization_tenant_rls_update" ON "organization" AS PERMISSIVE FOR UPDATE TO "app_tenant" USING ("organization"."id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("organization"."id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "organization_tenant_rls_delete" ON "organization" AS PERMISSIVE FOR DELETE TO "app_tenant" USING ("organization"."id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "organization_tenant_rls_insert" ON "organization" AS PERMISSIVE FOR INSERT TO "app_tenant" WITH CHECK (true);--> statement-breakpoint
CREATE POLICY "organization_invitation_tenant_rls" ON "organization_invitation" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("organization_invitation"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("organization_invitation"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "organization_member_tenant_rls" ON "organization_member" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("organization_member"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("organization_member"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));