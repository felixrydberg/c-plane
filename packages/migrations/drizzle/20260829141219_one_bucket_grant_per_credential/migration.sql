ALTER INDEX "bucket_grant_credential_bucket_prefix_uidx" RENAME TO "bucket_grant_credential_bucket_uidx";--> statement-breakpoint
DROP INDEX "bucket_grant_credential_bucket_uidx";--> statement-breakpoint
CREATE UNIQUE INDEX "bucket_grant_credential_bucket_uidx" ON "bucket_grant" ("credential_id","bucket_id");