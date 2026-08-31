CREATE TABLE "managed_registry_gc_runs" (
	"id" uuid PRIMARY KEY,
	"organization_id" uuid NOT NULL,
	"started_at" timestamp with time zone NOT NULL,
	"finished_at" timestamp with time zone NOT NULL,
	"bytes_before" bigint NOT NULL,
	"bytes_after" bigint NOT NULL
);
--> statement-breakpoint
ALTER TABLE "managed_registry_gc_runs" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE INDEX "managed_registry_gc_runs_organization_id_idx" ON "managed_registry_gc_runs" ("organization_id");--> statement-breakpoint
ALTER TABLE "managed_registry_gc_runs" ADD CONSTRAINT "managed_registry_gc_runs_HPcX8IPUKZjV_fkey" FOREIGN KEY ("organization_id") REFERENCES "managed_registry"("organization_id") ON DELETE CASCADE;--> statement-breakpoint
CREATE POLICY "managed_registry_gc_runs_tenant_rls" ON "managed_registry_gc_runs" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("managed_registry_gc_runs"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("managed_registry_gc_runs"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));