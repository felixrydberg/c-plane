CREATE TYPE "public"."s3_access_token_permission" AS ENUM('read_only', 'read_write');--> statement-breakpoint
CREATE TABLE "s3_access_token" (
	"id" uuid PRIMARY KEY NOT NULL,
	"organization_id" uuid NOT NULL,
	"project_id" uuid NOT NULL,
	"project_branch_id" uuid NOT NULL,
	"name" text NOT NULL,
	"access_key_id" text NOT NULL,
	"permission" "s3_access_token_permission" DEFAULT 'read_write' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"revoked_at" timestamp with time zone
);
--> statement-breakpoint
ALTER TABLE "s3_access_token" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "s3_access_token" ADD CONSTRAINT "s3_access_token_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "s3_access_token" ADD CONSTRAINT "s3_access_token_project_id_project_id_fk" FOREIGN KEY ("project_id") REFERENCES "public"."project"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "s3_access_token" ADD CONSTRAINT "s3_access_token_project_branch_id_project_branch_id_fk" FOREIGN KEY ("project_branch_id") REFERENCES "public"."project_branch"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE UNIQUE INDEX "s3_access_token_access_key_id_uidx" ON "s3_access_token" USING btree ("access_key_id");--> statement-breakpoint
CREATE UNIQUE INDEX "s3_access_token_branch_name_uidx" ON "s3_access_token" USING btree ("project_branch_id","name");--> statement-breakpoint
CREATE INDEX "s3_access_token_organization_id_idx" ON "s3_access_token" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "s3_access_token_project_id_idx" ON "s3_access_token" USING btree ("project_id");--> statement-breakpoint
CREATE INDEX "s3_access_token_project_branch_id_idx" ON "s3_access_token" USING btree ("project_branch_id");--> statement-breakpoint
CREATE POLICY "s3_access_token_tenant_rls" ON "s3_access_token" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("s3_access_token"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("s3_access_token"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));