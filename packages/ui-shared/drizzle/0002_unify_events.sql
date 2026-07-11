ALTER TABLE "event" ALTER COLUMN "type" SET DATA TYPE text USING "type"::text;--> statement-breakpoint
ALTER TABLE "event" ADD COLUMN "project_id" uuid;--> statement-breakpoint
ALTER TABLE "event" ADD COLUMN "actor_id" uuid;--> statement-breakpoint
CREATE INDEX "event_project_idx" ON "event" USING btree ("project_id","created_at");--> statement-breakpoint
DROP TYPE "public"."event_types";
