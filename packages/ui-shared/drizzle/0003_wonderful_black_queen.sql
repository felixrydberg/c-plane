CREATE TYPE "public"."bucket_status" AS ENUM('provisioning', 'ready', 'deleting', 'failed');--> statement-breakpoint
ALTER TABLE "bucket" ADD COLUMN "status" "bucket_status" DEFAULT 'provisioning' NOT NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "bucket_name_idx" ON "bucket" USING btree ("name");
