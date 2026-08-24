ALTER TABLE "worker_job" RENAME TO "worker_queue";--> statement-breakpoint
ALTER TABLE "worker_queue" DROP CONSTRAINT "worker_job_status_check";--> statement-breakpoint
ALTER TABLE "worker_queue" DROP CONSTRAINT "worker_job_attempts_check";--> statement-breakpoint
DROP INDEX "worker_job_organization_id_idx";--> statement-breakpoint
DROP INDEX "worker_job_claim_idx";--> statement-breakpoint
DROP INDEX "worker_job_lease_idx";--> statement-breakpoint
DROP INDEX "worker_job_active_dedupe_uidx";--> statement-breakpoint
ALTER TABLE "bucket" DROP COLUMN "status";--> statement-breakpoint
CREATE INDEX "worker_queue_organization_id_idx" ON "worker_queue" ("organization_id");--> statement-breakpoint
CREATE INDEX "worker_queue_claim_idx" ON "worker_queue" ("queue_name","status","available_at","created_at");--> statement-breakpoint
CREATE INDEX "worker_queue_lease_idx" ON "worker_queue" ("status","lease_expires_at");--> statement-breakpoint
CREATE UNIQUE INDEX "worker_queue_active_dedupe_uidx" ON "worker_queue" ("queue_name","dedupe_key") WHERE "dedupe_key" is not null and "status" in ('queued', 'running');--> statement-breakpoint
ALTER TABLE "worker_queue" ADD CONSTRAINT "worker_queue_status_check" CHECK ("status" in ('queued', 'running', 'succeeded', 'failed'));--> statement-breakpoint
ALTER TABLE "worker_queue" ADD CONSTRAINT "worker_queue_attempts_check" CHECK ("attempts" >= 0 and "max_attempts" > 0);--> statement-breakpoint
DROP TYPE "bucket_status";