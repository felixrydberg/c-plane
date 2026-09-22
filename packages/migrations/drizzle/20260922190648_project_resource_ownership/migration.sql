ALTER TABLE "bucket" ADD COLUMN "project_id" uuid;
--> statement-breakpoint
ALTER TABLE "credential" ADD COLUMN "project_id" uuid;
--> statement-breakpoint
ALTER TABLE "secret" ADD COLUMN "project_id" uuid;
--> statement-breakpoint
CREATE INDEX "bucket_project_id_idx" ON "bucket" ("project_id");
--> statement-breakpoint
CREATE INDEX "credential_project_id_idx" ON "credential" ("project_id");
--> statement-breakpoint
CREATE INDEX "secret_project_id_idx" ON "secret" ("project_id");
--> statement-breakpoint
ALTER TABLE "bucket" ADD CONSTRAINT "bucket_project_id_project_id_fkey" FOREIGN KEY ("project_id") REFERENCES "project"("id") ON DELETE CASCADE;
--> statement-breakpoint
ALTER TABLE "credential" ADD CONSTRAINT "credential_project_id_project_id_fkey" FOREIGN KEY ("project_id") REFERENCES "project"("id") ON DELETE CASCADE;
--> statement-breakpoint
ALTER TABLE "secret" ADD CONSTRAINT "secret_project_id_project_id_fkey" FOREIGN KEY ("project_id") REFERENCES "project"("id") ON DELETE CASCADE;