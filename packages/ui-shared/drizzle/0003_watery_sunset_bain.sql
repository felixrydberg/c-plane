CREATE TYPE "public"."region_routing_mode" AS ENUM('active', 'draining', 'disabled');--> statement-breakpoint
ALTER TABLE "region_s3_providers" DISABLE ROW LEVEL SECURITY;--> statement-breakpoint
DROP TABLE "region_s3_providers" CASCADE;--> statement-breakpoint
DROP INDEX "organization_s3_buckets_org_region_bucket_uidx";--> statement-breakpoint
ALTER TABLE "regions" ADD COLUMN "s3_provider_id" uuid;--> statement-breakpoint
ALTER TABLE "regions" ADD COLUMN "routing_mode" "region_routing_mode" DEFAULT 'active' NOT NULL;--> statement-breakpoint
ALTER TABLE "clusters" ADD COLUMN "ingress_endpoint" text;--> statement-breakpoint
ALTER TABLE "clusters" ADD COLUMN "ingress_enabled" boolean DEFAULT true NOT NULL;--> statement-breakpoint
ALTER TABLE "clusters" ADD COLUMN "ingress_weight" integer DEFAULT 100 NOT NULL;--> statement-breakpoint
ALTER TABLE "regions" ADD CONSTRAINT "regions_s3_provider_id_s3_providers_id_fk" FOREIGN KEY ("s3_provider_id") REFERENCES "public"."s3_providers"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "regions_routing_mode_idx" ON "regions" USING btree ("routing_mode");--> statement-breakpoint
CREATE INDEX "regions_s3_provider_id_idx" ON "regions" USING btree ("s3_provider_id");--> statement-breakpoint
CREATE INDEX "clusters_ingress_enabled_idx" ON "clusters" USING btree ("ingress_enabled");--> statement-breakpoint
CREATE INDEX "clusters_ingress_weight_idx" ON "clusters" USING btree ("ingress_weight");--> statement-breakpoint
CREATE UNIQUE INDEX "organization_s3_buckets_org_bucket_uidx" ON "organization_s3_buckets" USING btree ("organization_id","bucket_name");