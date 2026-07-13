CREATE TABLE "s3_access_token_bucket" (
	"access_token_id" uuid NOT NULL,
	"bucket_id" uuid NOT NULL,
	"organization_id" uuid NOT NULL,
	"can_read" boolean DEFAULT false NOT NULL,
	"can_write" boolean DEFAULT false NOT NULL
);
--> statement-breakpoint
ALTER TABLE "s3_access_token_bucket" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "s3_access_token_bucket" ADD CONSTRAINT "s3_access_token_bucket_access_token_id_s3_access_token_id_fk" FOREIGN KEY ("access_token_id") REFERENCES "public"."s3_access_token"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "s3_access_token_bucket" ADD CONSTRAINT "s3_access_token_bucket_bucket_id_bucket_id_fk" FOREIGN KEY ("bucket_id") REFERENCES "public"."bucket"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "s3_access_token_bucket" ADD CONSTRAINT "s3_access_token_bucket_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE UNIQUE INDEX "s3_access_token_bucket_uidx" ON "s3_access_token_bucket" USING btree ("access_token_id","bucket_id");--> statement-breakpoint
CREATE INDEX "s3_access_token_bucket_token_id_idx" ON "s3_access_token_bucket" USING btree ("access_token_id");--> statement-breakpoint
CREATE INDEX "s3_access_token_bucket_bucket_id_idx" ON "s3_access_token_bucket" USING btree ("bucket_id");--> statement-breakpoint
CREATE POLICY "s3_access_token_bucket_tenant_rls" ON "s3_access_token_bucket" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("s3_access_token_bucket"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("s3_access_token_bucket"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));