CREATE TABLE "ui_studio_account" (
	"id" uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid() NOT NULL,
	"account_id" text NOT NULL,
	"provider_id" text NOT NULL,
	"user_id" uuid NOT NULL,
	"access_token" text,
	"refresh_token" text,
	"id_token" text,
	"access_token_expires_at" timestamp,
	"refresh_token_expires_at" timestamp,
	"scope" text,
	"password" text,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp NOT NULL
);
--> statement-breakpoint
CREATE TABLE "ui_studio_auth_verification" (
	"id" uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid() NOT NULL,
	"identifier" text NOT NULL,
	"value" text NOT NULL,
	"expires_at" timestamp NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "ui_studio_two_factor" (
	"id" uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid() NOT NULL,
	"secret" text NOT NULL,
	"backup_codes" text NOT NULL,
	"user_id" uuid NOT NULL
);
--> statement-breakpoint
CREATE TABLE "ui_studio_user" (
	"id" uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid() NOT NULL,
	"name" text NOT NULL,
	"email" text NOT NULL,
	"email_verified" boolean DEFAULT false NOT NULL,
	"image" text,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"role" text,
	"banned" boolean DEFAULT false,
	"ban_reason" text,
	"ban_expires" timestamp,
	"two_factor_enabled" boolean DEFAULT false,
	CONSTRAINT "ui_studio_user_email_unique" UNIQUE("email")
);
--> statement-breakpoint
ALTER TABLE "ui_studio_account" ADD CONSTRAINT "ui_studio_account_user_id_ui_studio_user_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."ui_studio_user"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "ui_studio_two_factor" ADD CONSTRAINT "ui_studio_two_factor_user_id_ui_studio_user_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."ui_studio_user"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "ui_studio_account_userId_idx" ON "ui_studio_account" USING btree ("user_id");--> statement-breakpoint
CREATE INDEX "ui_studio_auth_verification_identifier_idx" ON "ui_studio_auth_verification" USING btree ("identifier");--> statement-breakpoint
CREATE INDEX "ui_studio_two_factor_secret_idx" ON "ui_studio_two_factor" USING btree ("secret");--> statement-breakpoint
CREATE INDEX "ui_studio_two_factor_userId_idx" ON "ui_studio_two_factor" USING btree ("user_id");