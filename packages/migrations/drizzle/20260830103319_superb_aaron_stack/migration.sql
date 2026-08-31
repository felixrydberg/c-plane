CREATE TYPE "managed_registry_status" AS ENUM('active', 'maintenance');--> statement-breakpoint
DELETE FROM "worker_queue" WHERE "job_type" = 'registry_gc' AND "organization_id" IS NULL;--> statement-breakpoint
DROP TABLE "registry_maintenance";--> statement-breakpoint
ALTER TABLE "managed_registry" ADD COLUMN "status" "managed_registry_status" DEFAULT 'active'::"managed_registry_status" NOT NULL;--> statement-breakpoint
ALTER TABLE "managed_registry" ADD COLUMN "gc_schedule_enabled" boolean DEFAULT true NOT NULL;--> statement-breakpoint
ALTER TABLE "managed_registry" ADD COLUMN "gc_schedule_local_time" time DEFAULT '03:00:00' NOT NULL;--> statement-breakpoint
ALTER TABLE "managed_registry" ADD COLUMN "gc_schedule_timezone" text DEFAULT 'Etc/UTC' NOT NULL;--> statement-breakpoint
ALTER TABLE "managed_registry" ADD COLUMN "gc_active_job_id" uuid;--> statement-breakpoint
ALTER TABLE "managed_registry" ADD COLUMN "gc_started_at" timestamp with time zone;--> statement-breakpoint
ALTER TABLE "managed_registry" ADD COLUMN "gc_finished_at" timestamp with time zone;--> statement-breakpoint
ALTER TABLE "managed_registry" ADD COLUMN "gc_last_result" text;--> statement-breakpoint
ALTER TABLE "managed_registry" ADD COLUMN "gc_last_error" text;--> statement-breakpoint
WITH "schedule" AS (
	SELECT "organization_id", "gc_schedule_local_time" AS "local_time", "gc_schedule_timezone" AS "timezone", NOW() AS "current_time", (NOW() AT TIME ZONE "gc_schedule_timezone")::date AS "local_date"
	FROM "managed_registry"
), "inserted" AS (
	INSERT INTO "worker_queue" ("id", "organization_id", "queue_name", "job_type", "dedupe_key", "payload", "available_at")
	SELECT gen_random_uuid(), "organization_id", 'maintenance', 'registry_gc', 'registry_gc:' || "organization_id"::text, jsonb_build_object('trigger', 'schedule'),
		CASE WHEN (("local_date" + "local_time") AT TIME ZONE "timezone") > "current_time"
			THEN (("local_date" + "local_time") AT TIME ZONE "timezone")
			ELSE (("local_date" + 1 + "local_time") AT TIME ZONE "timezone") END
	FROM "schedule"
	RETURNING "id", "organization_id"
)
UPDATE "managed_registry" SET "gc_active_job_id" = "inserted"."id", "updated_at" = NOW()
FROM "inserted" WHERE "managed_registry"."organization_id" = "inserted"."organization_id";--> statement-breakpoint
ALTER TABLE "managed_registry" DROP COLUMN "gc_active";--> statement-breakpoint
CREATE INDEX "managed_registry_status_idx" ON "managed_registry" ("status");--> statement-breakpoint
CREATE INDEX "managed_registry_gc_active_job_idx" ON "managed_registry" ("gc_active_job_id");--> statement-breakpoint
ALTER TABLE "managed_registry" ADD CONSTRAINT "managed_registry_gc_active_job_id_worker_queue_id_fkey" FOREIGN KEY ("gc_active_job_id") REFERENCES "worker_queue"("id") ON DELETE SET NULL;--> statement-breakpoint
CREATE POLICY "worker_queue_tenant_rls" ON "worker_queue" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("worker_queue"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("worker_queue"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));
