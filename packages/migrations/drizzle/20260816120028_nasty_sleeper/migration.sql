ALTER TABLE "project_container" RENAME TO "container";--> statement-breakpoint
ALTER TABLE "project_container_version" RENAME TO "container_version";--> statement-breakpoint
DROP POLICY "project_container_tenant_rls" ON "container";--> statement-breakpoint
DROP POLICY "project_container_version_tenant_rls" ON "container_version";--> statement-breakpoint
DROP POLICY "worker_job_external_registry_cleanup_insert_rls" ON "worker_job";--> statement-breakpoint
ALTER TABLE "container_version" DROP CONSTRAINT "project_container_version_external_registry_fk";--> statement-breakpoint
ALTER INDEX "project_container_organization_id_idx" RENAME TO "container_organization_id_idx";--> statement-breakpoint
ALTER INDEX "project_container_project_id_idx" RENAME TO "container_project_id_idx";--> statement-breakpoint
DROP INDEX "project_container_version_container_id_version_uidx";--> statement-breakpoint
DROP INDEX "project_container_version_container_id_idx";--> statement-breakpoint
DROP INDEX "project_container_version_organization_id_idx";--> statement-breakpoint
DROP INDEX "project_container_version_external_registry_id_idx";--> statement-breakpoint
CREATE UNIQUE INDEX "container_version_container_id_version_uidx" ON "container_version" ("container_id","version");--> statement-breakpoint
CREATE INDEX "container_version_container_id_idx" ON "container_version" ("container_id");--> statement-breakpoint
CREATE INDEX "container_version_organization_id_idx" ON "container_version" ("organization_id");--> statement-breakpoint
CREATE INDEX "container_version_external_registry_id_idx" ON "container_version" ("external_registry_id");--> statement-breakpoint
ALTER TABLE "container_version" ADD CONSTRAINT "container_version_external_registry_fk" FOREIGN KEY ("external_registry_id","organization_id") REFERENCES "external_registry"("id","organization_id") ON DELETE RESTRICT;--> statement-breakpoint
CREATE POLICY "container_tenant_rls" ON "container" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("container"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("container"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "container_version_tenant_rls" ON "container_version" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("container_version"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("container_version"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));