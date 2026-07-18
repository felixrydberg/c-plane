CREATE TABLE "registry_storage" (
	"id" uuid PRIMARY KEY NOT NULL,
	"service" text DEFAULT 'distribution' NOT NULL,
	"provider_id" uuid NOT NULL,
	"bucket_name" text NOT NULL,
	"physical_bucket_name" text NOT NULL,
	"access_key_id" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "registry_storage" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "registry_storage" ADD CONSTRAINT "registry_storage_provider_id_s3_providers_id_fk" FOREIGN KEY ("provider_id") REFERENCES "public"."s3_providers"("id") ON DELETE restrict ON UPDATE no action;--> statement-breakpoint
CREATE UNIQUE INDEX "registry_storage_service_uidx" ON "registry_storage" USING btree ("service");--> statement-breakpoint
CREATE UNIQUE INDEX "registry_storage_bucket_name_uidx" ON "registry_storage" USING btree ("bucket_name");--> statement-breakpoint
CREATE UNIQUE INDEX "registry_storage_physical_bucket_name_uidx" ON "registry_storage" USING btree ("physical_bucket_name");--> statement-breakpoint
CREATE UNIQUE INDEX "registry_storage_access_key_id_uidx" ON "registry_storage" USING btree ("access_key_id");--> statement-breakpoint
CREATE INDEX "registry_storage_provider_id_idx" ON "registry_storage" USING btree ("provider_id");