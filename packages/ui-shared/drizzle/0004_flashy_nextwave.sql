CREATE TABLE "registry_access_tokens" (
	"id" uuid PRIMARY KEY NOT NULL,
	"organization_id" uuid NOT NULL,
	"name" text NOT NULL,
	"token_hash" text NOT NULL,
	"token_prefix" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"revoked_at" timestamp with time zone
);
--> statement-breakpoint
ALTER TABLE "registry_access_tokens" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "registry_repositories" (
	"id" uuid PRIMARY KEY NOT NULL,
	"organization_id" uuid NOT NULL,
	"name" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "registry_repositories" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE TABLE "registry_repository_grants" (
	"id" uuid PRIMARY KEY NOT NULL,
	"organization_id" uuid NOT NULL,
	"repository_id" uuid NOT NULL,
	"access_token_id" uuid NOT NULL,
	"can_pull" boolean DEFAULT false NOT NULL,
	"can_push" boolean DEFAULT false NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "registry_repository_grants" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "registry_access_tokens" ADD CONSTRAINT "registry_access_tokens_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "registry_repositories" ADD CONSTRAINT "registry_repositories_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "registry_repository_grants" ADD CONSTRAINT "registry_repository_grants_organization_id_organization_id_fk" FOREIGN KEY ("organization_id") REFERENCES "public"."organization"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "registry_repository_grants" ADD CONSTRAINT "registry_repository_grants_repository_id_registry_repositories_id_fk" FOREIGN KEY ("repository_id") REFERENCES "public"."registry_repositories"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "registry_repository_grants" ADD CONSTRAINT "registry_repository_grants_access_token_id_registry_access_tokens_id_fk" FOREIGN KEY ("access_token_id") REFERENCES "public"."registry_access_tokens"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE UNIQUE INDEX "registry_access_tokens_hash_uidx" ON "registry_access_tokens" USING btree ("token_hash");--> statement-breakpoint
CREATE UNIQUE INDEX "registry_access_tokens_organization_name_uidx" ON "registry_access_tokens" USING btree ("organization_id","name") WHERE "registry_access_tokens"."revoked_at" is null;--> statement-breakpoint
CREATE INDEX "registry_access_tokens_organization_id_idx" ON "registry_access_tokens" USING btree ("organization_id");--> statement-breakpoint
CREATE UNIQUE INDEX "registry_repositories_organization_name_uidx" ON "registry_repositories" USING btree ("organization_id","name");--> statement-breakpoint
CREATE INDEX "registry_repositories_organization_id_idx" ON "registry_repositories" USING btree ("organization_id");--> statement-breakpoint
CREATE UNIQUE INDEX "registry_repository_grants_token_repository_uidx" ON "registry_repository_grants" USING btree ("access_token_id","repository_id");--> statement-breakpoint
CREATE INDEX "registry_repository_grants_organization_id_idx" ON "registry_repository_grants" USING btree ("organization_id");--> statement-breakpoint
CREATE INDEX "registry_repository_grants_repository_id_idx" ON "registry_repository_grants" USING btree ("repository_id");--> statement-breakpoint
CREATE INDEX "registry_repository_grants_access_token_id_idx" ON "registry_repository_grants" USING btree ("access_token_id");--> statement-breakpoint
CREATE POLICY "registry_access_tokens_tenant_rls" ON "registry_access_tokens" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("registry_access_tokens"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("registry_access_tokens"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "registry_repositories_tenant_rls" ON "registry_repositories" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("registry_repositories"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("registry_repositories"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));--> statement-breakpoint
CREATE POLICY "registry_repository_grants_tenant_rls" ON "registry_repository_grants" AS PERMISSIVE FOR ALL TO "app_tenant" USING ("registry_repository_grants"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))) WITH CHECK ("registry_repository_grants"."organization_id" = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[])));