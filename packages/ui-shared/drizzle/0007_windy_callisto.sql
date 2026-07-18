DROP POLICY "serverless_postgres_database_tenant_rls" ON "serverless_postgres_database" CASCADE;--> statement-breakpoint
DROP TABLE "serverless_postgres_database" CASCADE;--> statement-breakpoint
DROP POLICY "serverless_postgres_database_branch_tenant_rls" ON "serverless_postgres_database_branch" CASCADE;--> statement-breakpoint
DROP TABLE "serverless_postgres_database_branch" CASCADE;--> statement-breakpoint
ALTER TABLE "organization" DROP CONSTRAINT "organization_polar_customer_id_unique";--> statement-breakpoint
DROP INDEX "organization_polar_customer_id_idx";--> statement-breakpoint
ALTER TABLE "organization" DROP COLUMN "polar_customer_id";