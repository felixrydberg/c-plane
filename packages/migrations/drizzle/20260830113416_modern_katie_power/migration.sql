ALTER TABLE "managed_registry_gc_runs" ADD COLUMN "result" text NOT NULL;--> statement-breakpoint
ALTER TABLE "managed_registry_gc_runs" ADD COLUMN "error" text;--> statement-breakpoint
ALTER TABLE "managed_registry" DROP COLUMN "gc_started_at";--> statement-breakpoint
ALTER TABLE "managed_registry" DROP COLUMN "gc_finished_at";--> statement-breakpoint
ALTER TABLE "managed_registry" DROP COLUMN "gc_last_result";--> statement-breakpoint
ALTER TABLE "managed_registry" DROP COLUMN "gc_last_error";--> statement-breakpoint
ALTER TABLE "managed_registry_gc_runs" ALTER COLUMN "bytes_before" DROP NOT NULL;--> statement-breakpoint
ALTER TABLE "managed_registry_gc_runs" ALTER COLUMN "bytes_after" DROP NOT NULL;