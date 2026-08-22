CREATE TYPE "organization_member_permission_scope_type" AS ENUM('region:read', 'project:read', 'project:create', 'project:delete', 'project:manage', 'access-token:read', 'access-token:create', 'access-token:update', 'access-token:delete', 'bucket:read', 'bucket:create', 'bucket:delete', 'timeline:read', 'event:read', 'container:read', 'container:create', 'container:update', 'container:delete', 'database:postgres:read', 'database:postgres:create', 'database:postgres:update', 'database:postgres:delete', 'database:postgres:manage', 'registry:read', 'registry:create', 'registry:update', 'registry:delete', 'org:update', 'member:invite', 'member:remove', 'api-key:manage');--> statement-breakpoint
CREATE TABLE "organization_member_permission" (
	"id" uuid PRIMARY KEY,
	"member_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"scope" "organization_member_permission_scope_type" NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "organization_member_permission" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE UNIQUE INDEX "organization_member_permission_member_id_scope_uidx" ON "organization_member_permission" ("member_id","scope");--> statement-breakpoint
CREATE INDEX "organization_member_permission_organization_id_idx" ON "organization_member_permission" ("organization_id");--> statement-breakpoint
CREATE INDEX "organization_member_permission_member_id_idx" ON "organization_member_permission" ("member_id");--> statement-breakpoint
ALTER TABLE "organization_member_permission" ADD CONSTRAINT "organization_member_permission_lSLy26ie0K9a_fkey" FOREIGN KEY ("member_id") REFERENCES "organization_member"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "organization_member_permission" ADD CONSTRAINT "organization_member_permission_DCAxdTDApsJo_fkey" FOREIGN KEY ("organization_id") REFERENCES "organization"("id") ON DELETE CASCADE;--> statement-breakpoint
CREATE POLICY "organization_member_permission_tenant_rls" ON "organization_member_permission" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("organization_member_permission"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("organization_member_permission"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));