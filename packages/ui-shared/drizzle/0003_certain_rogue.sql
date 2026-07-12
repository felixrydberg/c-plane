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
	"is_public" boolean DEFAULT false NOT NULL
);
--> statement-breakpoint
ALTER TABLE "bucket" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
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
CREATE INDEX "bucket_project_id_idx" ON "bucket" USING btree ("project_id");--> statement-breakpoint
CREATE INDEX "bucket_organization_id_idx" ON "bucket" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "bucket_region_idx" ON "bucket" USING btree ("region");--> statement-breakpoint
ALTER TABLE "s3_providers" DROP COLUMN "access_key_id";--> statement-breakpoint
ALTER TABLE "s3_providers" DROP COLUMN "secret_access_key_encrypted";--> statement-breakpoint
ALTER TABLE "s3_providers" DROP COLUMN "session_token_encrypted";--> statement-breakpoint
CREATE POLICY "serverless_postgres_database_tenant_rls" ON "serverless_postgres_database" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("serverless_postgres_database"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("serverless_postgres_database"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "serverless_postgres_database_branch_tenant_rls" ON "serverless_postgres_database_branch" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("serverless_postgres_database_branch"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("serverless_postgres_database_branch"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "stateful_postgres_database_tenant_rls" ON "stateful_postgres_database" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("stateful_postgres_database"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("stateful_postgres_database"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "stateful_postgres_database_branch_tenant_rls" ON "stateful_postgres_database_branch" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("stateful_postgres_database_branch"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("stateful_postgres_database_branch"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "bucket_tenant_rls" ON "bucket" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("bucket"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("bucket"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));