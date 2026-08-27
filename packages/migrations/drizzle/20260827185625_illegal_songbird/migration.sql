ALTER TABLE "s3_providers" RENAME COLUMN "provider_type" TO "name";--> statement-breakpoint
ALTER INDEX "s3_providers_provider_type_idx" RENAME TO "s3_providers_name_idx";--> statement-breakpoint
ALTER TABLE "s3_providers" ALTER COLUMN "name" SET DATA TYPE text USING "name"::text;--> statement-breakpoint
DROP TYPE "s3_provider_type";