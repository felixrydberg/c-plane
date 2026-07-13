ALTER TABLE "s3_access_token" DROP CONSTRAINT "s3_access_token_project_branch_id_project_branch_id_fk";
--> statement-breakpoint
DROP INDEX "s3_access_token_branch_name_uidx";--> statement-breakpoint
DROP INDEX "s3_access_token_project_branch_id_idx";--> statement-breakpoint
CREATE UNIQUE INDEX "s3_access_token_project_name_uidx" ON "s3_access_token" USING btree ("project_id","name") WHERE "s3_access_token"."revoked_at" is null;--> statement-breakpoint
ALTER TABLE "s3_access_token" DROP COLUMN "project_branch_id";--> statement-breakpoint
ALTER TABLE "s3_access_token" DROP COLUMN "permission";--> statement-breakpoint
DROP TYPE "public"."s3_access_token_permission";