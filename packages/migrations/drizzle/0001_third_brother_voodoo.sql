ALTER TABLE "project_container" RENAME TO "container";--> statement-breakpoint
ALTER TABLE "project_container_version" RENAME TO "container_version";--> statement-breakpoint
ALTER TABLE "container" DROP CONSTRAINT "project_container_project_id_project_id_fk";
--> statement-breakpoint
ALTER TABLE "container" DROP CONSTRAINT "project_container_organization_id_organization_id_fk";
--> statement-breakpoint
ALTER TABLE "container" DROP CONSTRAINT "project_container_region_id_regions_id_fk";
--> statement-breakpoint
ALTER TABLE "container_version" DROP CONSTRAINT "project_container_version_container_id_project_container_id_fk";
--> statement-breakpoint
ALTER TABLE "container_version" DROP CONSTRAINT "project_container_version_organization_id_organization_id_fk";
--> statement-breakpoint
ALTER TABLE "container_version" DROP CONSTRAINT "project_container_version_external_registry_fk";
--> statement-breakpoint
DROP INDEX "project_container_organization_id_idx";--> statement-breakpoint
DROP INDEX "project_container_project_id_idx";--> statement-breakpoint
DROP INDEX "project_container_version_container_id_version_uidx";--> statement-breakpoint
DROP INDEX "project_container_version_container_id_idx";--> statement-breakpoint
DROP INDEX "project_container_version_organization_id_idx";--> statement-breakpoint
DROP INDEX "project_container_version_external_registry_id_idx";--> statement-breakpoint
ALTER TABLE "container" ADD CONSTRAINT "container_project_id_project_id_fk" FOREIGN KEY ("project_id") REFERENCES "public"."project"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "container" ADD CONSTRAINT "container_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "container" ADD CONSTRAINT "container_region_id_regions_id_fk" FOREIGN KEY ("region_id") REFERENCES "public"."regions"("id") ON DELETE restrict ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "container_version" ADD CONSTRAINT "container_version_container_id_container_id_fk" FOREIGN KEY ("container_id") REFERENCES "public"."container"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "container_version" ADD CONSTRAINT "container_version_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "container_version" ADD CONSTRAINT "container_version_external_registry_fk" FOREIGN KEY ("external_registry_id","organization_id") REFERENCES "public"."external_registry"("id","organization_id") ON DELETE restrict ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "container_organization_id_idx" ON "container" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "container_project_id_idx" ON "container" USING btree ("project_id");--> statement-breakpoint
CREATE UNIQUE INDEX "container_version_container_id_version_uidx" ON "container_version" USING btree ("container_id","version");--> statement-breakpoint
CREATE INDEX "container_version_container_id_idx" ON "container_version" USING btree ("container_id");--> statement-breakpoint
CREATE INDEX "container_version_organization_id_idx" ON "container_version" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "container_version_external_registry_id_idx" ON "container_version" USING btree ("external_registry_id");--> statement-breakpoint
ALTER POLICY "project_container_tenant_rls" ON "container" RENAME TO "container_tenant_rls";--> statement-breakpoint
ALTER POLICY "project_container_version_tenant_rls" ON "container_version" RENAME TO "container_version_tenant_rls";