CREATE TABLE "storage_bucket_name_reservation" (
    "organization_id" uuid NOT NULL REFERENCES "organization"("id") ON DELETE CASCADE,
    "project_id" uuid NOT NULL REFERENCES "project"("id") ON DELETE CASCADE,
    "name" text NOT NULL,
    "bucket_id" uuid NOT NULL,
    "region_id" uuid NOT NULL REFERENCES "regions"("id") ON DELETE RESTRICT,
    "provider_id" uuid NOT NULL REFERENCES "s3_providers"("id") ON DELETE RESTRICT,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT "storage_bucket_name_reservation_pk" PRIMARY KEY ("project_id", "name")
);
--> statement-breakpoint
CREATE INDEX "storage_bucket_name_reservation_organization_id_idx" ON "storage_bucket_name_reservation" ("organization_id");
--> statement-breakpoint
ALTER TABLE "storage_bucket_name_reservation" ENABLE ROW LEVEL SECURITY;
--> statement-breakpoint
CREATE POLICY "storage_bucket_name_reservation_tenant_rls" ON "storage_bucket_name_reservation" AS PERMISSIVE FOR ALL TO "app_tenant"
USING ("storage_bucket_name_reservation"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])))
WITH CHECK ("storage_bucket_name_reservation"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));
--> statement-breakpoint
CREATE TABLE "managed_registry_activation_reservation" (
    "organization_id" uuid PRIMARY KEY REFERENCES "organization"("id") ON DELETE CASCADE,
    "bucket_id" uuid NOT NULL,
    "region_id" uuid NOT NULL REFERENCES "regions"("id") ON DELETE RESTRICT,
    "provider_id" uuid NOT NULL REFERENCES "s3_providers"("id") ON DELETE RESTRICT,
    "created_at" timestamptz NOT NULL DEFAULT now()
);
--> statement-breakpoint
ALTER TABLE "managed_registry_activation_reservation" ENABLE ROW LEVEL SECURITY;
--> statement-breakpoint
CREATE POLICY "managed_registry_activation_reservation_tenant_rls" ON "managed_registry_activation_reservation" AS PERMISSIVE FOR ALL TO "app_tenant"
USING ("managed_registry_activation_reservation"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])))
WITH CHECK ("managed_registry_activation_reservation"."organization_id" = ANY(COALESCE(NULLIF(current_setting('app.allowed_organizations', true), '')::uuid[], ARRAY[]::uuid[])));
