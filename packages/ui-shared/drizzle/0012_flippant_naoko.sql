CREATE TABLE "registry_maintenance" (
	"service" text PRIMARY KEY DEFAULT 'distribution' NOT NULL,
	"gc_access_key_id" text NOT NULL,
	"phase" text DEFAULT 'idle' NOT NULL,
	"active_job_id" uuid,
	"started_at" timestamp with time zone,
	"finished_at" timestamp with time zone,
	"last_result" text,
	"last_error" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "registry_maintenance_gc_access_key_id_unique" UNIQUE("gc_access_key_id"),
	CONSTRAINT "registry_maintenance_phase_check" CHECK ("registry_maintenance"."phase" in ('idle', 'queued', 'draining', 'collecting', 'restoring'))
);
--> statement-breakpoint
ALTER TABLE "registry_maintenance" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "worker_job" (
	"id" uuid PRIMARY KEY NOT NULL,
	"queue_name" text NOT NULL,
	"job_type" text NOT NULL,
	"payload" jsonb DEFAULT '{}'::jsonb NOT NULL,
	"status" text DEFAULT 'queued' NOT NULL,
	"dedupe_key" text,
	"attempts" integer DEFAULT 0 NOT NULL,
	"max_attempts" integer DEFAULT 3 NOT NULL,
	"available_at" timestamp with time zone DEFAULT now() NOT NULL,
	"locked_by" text,
	"lease_expires_at" timestamp with time zone,
	"last_error" text,
	"started_at" timestamp with time zone,
	"finished_at" timestamp with time zone,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "worker_job_status_check" CHECK ("worker_job"."status" in ('queued', 'running', 'succeeded', 'failed')),
	CONSTRAINT "worker_job_attempts_check" CHECK ("worker_job"."attempts" >= 0 and "worker_job"."max_attempts" > 0)
);
--> statement-breakpoint
ALTER TABLE "worker_job" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "registry_maintenance" ADD CONSTRAINT "registry_maintenance_active_job_id_worker_job_id_fk" FOREIGN KEY ("active_job_id") REFERENCES "public"."worker_job"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "registry_maintenance_active_job_idx" ON "registry_maintenance" USING btree ("active_job_id");--> statement-breakpoint
CREATE INDEX "worker_job_claim_idx" ON "worker_job" USING btree ("queue_name","status","available_at","created_at");--> statement-breakpoint
CREATE INDEX "worker_job_lease_idx" ON "worker_job" USING btree ("status","lease_expires_at");--> statement-breakpoint
CREATE UNIQUE INDEX "worker_job_active_dedupe_uidx" ON "worker_job" USING btree ("queue_name","dedupe_key") WHERE "worker_job"."dedupe_key" is not null and "worker_job"."status" in ('queued', 'running');