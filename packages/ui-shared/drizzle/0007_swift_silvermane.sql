ALTER TYPE "public"."api_key_scopes_type" ADD VALUE 'registry:update' BEFORE 'registry:delete';--> statement-breakpoint
CREATE TABLE "external_registry" (
	"id" uuid PRIMARY KEY NOT NULL,
	"organization_id" uuid NOT NULL,
	"name" text NOT NULL,
	"host" text NOT NULL,
	"username" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "external_registry" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "worker_job" ADD COLUMN "organization_id" uuid;--> statement-breakpoint
ALTER TABLE "project_container_version" ADD COLUMN "external_registry_id" uuid;--> statement-breakpoint
ALTER TABLE "external_registry" ADD CONSTRAINT "external_registry_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE UNIQUE INDEX "external_registry_id_organization_id_uidx" ON "external_registry" USING btree ("id","organization_id");--> statement-breakpoint
CREATE UNIQUE INDEX "external_registry_organization_name_uidx" ON "external_registry" USING btree ("organization_id","name");--> statement-breakpoint
CREATE UNIQUE INDEX "external_registry_organization_host_username_uidx" ON "external_registry" USING btree ("organization_id","host","username");--> statement-breakpoint
CREATE INDEX "external_registry_organization_id_idx" ON "external_registry" USING btree ("organization_id");--> statement-breakpoint
ALTER TABLE "worker_job" ADD CONSTRAINT "worker_job_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "worker_job_organization_id_idx" ON "worker_job" USING btree ("organization_id");--> statement-breakpoint
ALTER TABLE "project_container_version" ADD CONSTRAINT "project_container_version_external_registry_fk" FOREIGN KEY ("external_registry_id","organization_id") REFERENCES "public"."external_registry"("id","organization_id") ON DELETE restrict ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "project_container_version_external_registry_id_idx" ON "project_container_version" USING btree ("external_registry_id");--> statement-breakpoint
ALTER TABLE "project_container_version" DROP COLUMN "pull_secret_id";--> statement-breakpoint
CREATE POLICY "worker_job_external_registry_cleanup_insert_rls" ON "worker_job" AS PERMISSIVE FOR INSERT TO "app_tenant" WITH CHECK ("worker_job"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])) and "worker_job"."queue_name" = 'secrets' and "worker_job"."job_type" = 'external_registry_secret_cleanup');--> statement-breakpoint
CREATE POLICY "external_registry_tenant_rls" ON "external_registry" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("external_registry"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("external_registry"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));
