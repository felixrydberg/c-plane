ALTER TABLE "project_environment" RENAME COLUMN "timeline" TO "draft_timeline";--> statement-breakpoint
ALTER TABLE "project_environment" DROP CONSTRAINT "project_environment_timeline_project_timeline_id_fk";
--> statement-breakpoint
ALTER TABLE "project_environment" ADD COLUMN "deployed_timeline" uuid;--> statement-breakpoint
UPDATE "project_environment" SET "deployed_timeline" = "draft_timeline";--> statement-breakpoint
ALTER TABLE "project_environment" ALTER COLUMN "deployed_timeline" SET NOT NULL;--> statement-breakpoint
ALTER TABLE "project_environment" ADD CONSTRAINT "project_environment_draft_timeline_project_timeline_id_fk" FOREIGN KEY ("draft_timeline") REFERENCES "public"."project_timeline"("id") ON DELETE restrict ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "project_environment" ADD CONSTRAINT "project_environment_deployed_timeline_project_timeline_id_fk" FOREIGN KEY ("deployed_timeline") REFERENCES "public"."project_timeline"("id") ON DELETE restrict ON UPDATE no action;
