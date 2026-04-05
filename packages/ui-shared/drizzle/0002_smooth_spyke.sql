CREATE TYPE "public"."organization_s3_bucket_status" AS ENUM('active', 'deleting', 'error');--> statement-breakpoint
CREATE TYPE "public"."s3_provider_type" AS ENUM('aws_s3', 'cloudflare_r2');--> statement-breakpoint
CREATE TABLE "cluster_join_credentials" (
	"id" uuid PRIMARY KEY NOT NULL,
	"cluster_id" uuid NOT NULL,
	"token_hash" text NOT NULL,
	"expires_at" timestamp with time zone NOT NULL,
	"used_at" timestamp with time zone,
	"revoked_at" timestamp with time zone,
	"revoked_reason" text,
	"issued_by_user_id" uuid,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "cluster_runtime_identities" (
	"id" uuid PRIMARY KEY NOT NULL,
	"cluster_id" uuid NOT NULL,
	"public_key_pem" text NOT NULL,
	"key_algorithm" text DEFAULT 'ed25519' NOT NULL,
	"key_version" integer DEFAULT 1 NOT NULL,
	"lease_epoch" integer DEFAULT 0 NOT NULL,
	"last_rotated_at" timestamp with time zone,
	"last_seen_at" timestamp with time zone,
	"revoked_at" timestamp with time zone,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "organization_s3_buckets" (
	"id" uuid PRIMARY KEY NOT NULL,
	"organization_id" uuid NOT NULL,
	"region_id" uuid NOT NULL,
	"provider_id" uuid NOT NULL,
	"bucket_name" text NOT NULL,
	"provider_bucket_name" text NOT NULL,
	"status" "organization_s3_bucket_status" DEFAULT 'active' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "region_s3_providers" (
	"id" uuid PRIMARY KEY NOT NULL,
	"region_id" uuid NOT NULL,
	"provider_id" uuid NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "s3_providers" (
	"id" uuid PRIMARY KEY NOT NULL,
	"provider_type" "s3_provider_type" NOT NULL,
	"endpoint_url" text NOT NULL,
	"provider_region" text,
	"access_key_id" text NOT NULL,
	"secret_access_key_encrypted" text NOT NULL,
	"session_token_encrypted" text,
	"is_active" boolean DEFAULT true NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "clusters" ALTER COLUMN "status" SET DATA TYPE text;--> statement-breakpoint
ALTER TABLE "clusters" ALTER COLUMN "status" SET DEFAULT 'pending'::text;--> statement-breakpoint
DROP TYPE "public"."cluster_status";--> statement-breakpoint
CREATE TYPE "public"."cluster_status" AS ENUM('pending', 'bootstrapping', 'healthy', 'draining', 'offline', 'removed');--> statement-breakpoint
ALTER TABLE "clusters" ALTER COLUMN "status" SET DEFAULT 'pending'::"public"."cluster_status";--> statement-breakpoint
ALTER TABLE "clusters" ALTER COLUMN "status" SET DATA TYPE "public"."cluster_status" USING "status"::"public"."cluster_status";--> statement-breakpoint
ALTER TABLE "cluster_join_credentials" ADD CONSTRAINT "cluster_join_credentials_cluster_id_clusters_id_fk" FOREIGN KEY ("cluster_id") REFERENCES "public"."clusters"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "cluster_join_credentials" ADD CONSTRAINT "cluster_join_credentials_issued_by_user_id_user_id_fk" FOREIGN KEY ("issued_by_user_id") REFERENCES "public"."user"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "cluster_runtime_identities" ADD CONSTRAINT "cluster_runtime_identities_cluster_id_clusters_id_fk" FOREIGN KEY ("cluster_id") REFERENCES "public"."clusters"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "organization_s3_buckets" ADD CONSTRAINT "organization_s3_buckets_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "organization_s3_buckets" ADD CONSTRAINT "organization_s3_buckets_region_id_regions_id_fk" FOREIGN KEY ("region_id") REFERENCES "public"."regions"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "organization_s3_buckets" ADD CONSTRAINT "organization_s3_buckets_provider_id_s3_providers_id_fk" FOREIGN KEY ("provider_id") REFERENCES "public"."s3_providers"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "region_s3_providers" ADD CONSTRAINT "region_s3_providers_region_id_regions_id_fk" FOREIGN KEY ("region_id") REFERENCES "public"."regions"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "region_s3_providers" ADD CONSTRAINT "region_s3_providers_provider_id_s3_providers_id_fk" FOREIGN KEY ("provider_id") REFERENCES "public"."s3_providers"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "cluster_join_credentials_cluster_id_idx" ON "cluster_join_credentials" USING btree ("cluster_id");--> statement-breakpoint
CREATE INDEX "cluster_join_credentials_expires_at_idx" ON "cluster_join_credentials" USING btree ("expires_at");--> statement-breakpoint
CREATE INDEX "cluster_join_credentials_token_hash_idx" ON "cluster_join_credentials" USING btree ("token_hash");--> statement-breakpoint
CREATE UNIQUE INDEX "cluster_runtime_identities_cluster_id_uidx" ON "cluster_runtime_identities" USING btree ("cluster_id");--> statement-breakpoint
CREATE INDEX "cluster_runtime_identities_revoked_at_idx" ON "cluster_runtime_identities" USING btree ("revoked_at");--> statement-breakpoint
CREATE UNIQUE INDEX "organization_s3_buckets_org_region_bucket_uidx" ON "organization_s3_buckets" USING btree ("organization_id","region_id","bucket_name");--> statement-breakpoint
CREATE UNIQUE INDEX "organization_s3_buckets_provider_bucket_uidx" ON "organization_s3_buckets" USING btree ("provider_id","provider_bucket_name");--> statement-breakpoint
CREATE INDEX "organization_s3_buckets_region_id_idx" ON "organization_s3_buckets" USING btree ("region_id");--> statement-breakpoint
CREATE INDEX "organization_s3_buckets_organization_id_idx" ON "organization_s3_buckets" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "organization_s3_buckets_provider_id_idx" ON "organization_s3_buckets" USING btree ("provider_id");--> statement-breakpoint
CREATE UNIQUE INDEX "region_s3_providers_region_id_uidx" ON "region_s3_providers" USING btree ("region_id");--> statement-breakpoint
CREATE INDEX "region_s3_providers_provider_id_idx" ON "region_s3_providers" USING btree ("provider_id");--> statement-breakpoint
CREATE INDEX "s3_providers_provider_type_idx" ON "s3_providers" USING btree ("provider_type");--> statement-breakpoint
CREATE INDEX "s3_providers_is_active_idx" ON "s3_providers" USING btree ("is_active");