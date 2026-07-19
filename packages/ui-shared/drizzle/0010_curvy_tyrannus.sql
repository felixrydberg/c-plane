ALTER TABLE "project_branch" RENAME TO "project_environment";--> statement-breakpoint
ALTER TABLE "project" RENAME COLUMN "default_branch_id" TO "default_environment_id";--> statement-breakpoint
ALTER TABLE "project_timeline" RENAME COLUMN "branch_id" TO "environment_id";--> statement-breakpoint
ALTER TABLE "project" DROP CONSTRAINT "project_default_branch_id_project_branch_id_fk";
--> statement-breakpoint
ALTER TABLE "project_environment" DROP CONSTRAINT "project_branch_project_id_project_id_fk";
--> statement-breakpoint
ALTER TABLE "project_environment" DROP CONSTRAINT "project_branch_organization_id_organization_id_fk";
--> statement-breakpoint
ALTER TABLE "project_environment" DROP CONSTRAINT "project_branch_timeline_project_timeline_id_fk";
--> statement-breakpoint
ALTER TABLE "postgres_database_branch" DROP CONSTRAINT "postgres_database_branch_branch_id_project_branch_id_fk";
--> statement-breakpoint
DROP INDEX "project_default_branch_id_idx";--> statement-breakpoint
DROP INDEX "project_branch_project_id_name_uidx";--> statement-breakpoint
DROP INDEX "project_branch_id_project_id_organization_id_uidx";--> statement-breakpoint
DROP INDEX "project_branch_organization_id_idx";--> statement-breakpoint
DROP INDEX "project_branch_project_id_idx";--> statement-breakpoint
DROP INDEX "project_timeline_branch_id_idx";--> statement-breakpoint
ALTER TABLE "project" ADD CONSTRAINT "project_default_environment_id_project_environment_id_fk" FOREIGN KEY ("default_environment_id") REFERENCES "public"."project_environment"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_environment" ADD CONSTRAINT "project_environment_project_id_project_id_fk" FOREIGN KEY ("project_id") REFERENCES "public"."project"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_environment" ADD CONSTRAINT "project_environment_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_environment" ADD CONSTRAINT "project_environment_timeline_project_timeline_id_fk" FOREIGN KEY ("timeline") REFERENCES "public"."project_timeline"("id") ON DELETE restrict ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "postgres_database_branch" ADD CONSTRAINT "postgres_database_branch_branch_id_project_environment_id_fk" FOREIGN KEY ("branch_id") REFERENCES "public"."project_environment"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "project_default_environment_id_idx" ON "project" USING btree ("default_environment_id");--> statement-breakpoint
CREATE UNIQUE INDEX "project_environment_project_id_name_uidx" ON "project_environment" USING btree ("project_id","name");--> statement-breakpoint
CREATE UNIQUE INDEX "project_environment_id_project_id_organization_id_uidx" ON "project_environment" USING btree ("id","project_id","organization_id");--> statement-breakpoint
CREATE INDEX "project_environment_organization_id_idx" ON "project_environment" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "project_environment_project_id_idx" ON "project_environment" USING btree ("project_id");--> statement-breakpoint
CREATE INDEX "project_timeline_environment_id_idx" ON "project_timeline" USING btree ("environment_id");--> statement-breakpoint
ALTER POLICY "project_branch_tenant_rls" ON "project_environment" RENAME TO "project_environment_tenant_rls";