ALTER TABLE "stateful_postgres_database_branch" RENAME TO "postgres_database_branch";--> statement-breakpoint
ALTER TABLE "stateful_postgres_database" RENAME TO "postgres_database";--> statement-breakpoint
ALTER TABLE "postgres_database" RENAME CONSTRAINT "stateful_postgres_database_project_id_project_id_fk" TO "postgres_database_project_id_project_id_fk";--> statement-breakpoint
ALTER TABLE "postgres_database" RENAME CONSTRAINT "stateful_postgres_database_organization_id_organization_id_fk" TO "postgres_database_organization_id_organization_id_fk";--> statement-breakpoint
ALTER TABLE "postgres_database" RENAME CONSTRAINT "stateful_postgres_database_default_branch_id_stateful_postgres_database_branch_id_fk" TO "postgres_database_default_branch_id_postgres_database_branch_id_fk";--> statement-breakpoint
ALTER TABLE "postgres_database_branch" RENAME CONSTRAINT "stateful_postgres_database_branch_database_id_stateful_postgres_database_id_fk" TO "postgres_database_branch_database_id_postgres_database_id_fk";--> statement-breakpoint
ALTER TABLE "postgres_database_branch" RENAME CONSTRAINT "stateful_postgres_database_branch_branch_id_project_branch_id_fk" TO "postgres_database_branch_branch_id_project_branch_id_fk";--> statement-breakpoint
ALTER TABLE "postgres_database_branch" RENAME CONSTRAINT "stateful_postgres_database_branch_organization_id_organization_id_fk" TO "postgres_database_branch_organization_id_organization_id_fk";--> statement-breakpoint
ALTER INDEX "stateful_postgres_database_project_id_idx" RENAME TO "postgres_database_project_id_idx";--> statement-breakpoint
ALTER INDEX "stateful_postgres_database_organization_id_idx" RENAME TO "postgres_database_organization_id_idx";--> statement-breakpoint
ALTER INDEX "stateful_postgres_database_branch_database_id_idx" RENAME TO "postgres_database_branch_database_id_idx";--> statement-breakpoint
ALTER INDEX "stateful_postgres_database_branch_branch_id_idx" RENAME TO "postgres_database_branch_branch_id_idx";--> statement-breakpoint
ALTER INDEX "stateful_postgres_database_branch_organization_id_idx" RENAME TO "postgres_database_branch_organization_id_idx";--> statement-breakpoint
ALTER POLICY "stateful_postgres_database_tenant_rls" ON "postgres_database" RENAME TO "postgres_database_tenant_rls";--> statement-breakpoint
ALTER POLICY "stateful_postgres_database_branch_tenant_rls" ON "postgres_database_branch" RENAME TO "postgres_database_branch_tenant_rls";
