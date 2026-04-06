import { sql } from "drizzle-orm";
import { boolean, index, pgEnum, pgPolicy, pgTable, text, timestamp, uniqueIndex, uuid } from "drizzle-orm/pg-core";
import { app_tenant } from "../rls";
import { region } from "../regions/schema";
import { organization } from "../organization/schema";

export const S3_PROVIDER_TYPES = ["aws_s3", "cloudflare_r2"] as const;
export const ORGANIZATION_S3_BUCKET_STATUSES = ["active", "deleting", "error"] as const;

export const s3_provider_type = pgEnum("s3_provider_type", S3_PROVIDER_TYPES);
export const organization_s3_bucket_status = pgEnum("organization_s3_bucket_status", ORGANIZATION_S3_BUCKET_STATUSES);

export const s3_provider = pgTable("s3_providers", {
  id: uuid("id").primaryKey(),
  provider_type: s3_provider_type("provider_type").notNull(),
  endpoint_url: text("endpoint_url").notNull(),
  provider_region: text("provider_region"),
  access_key_id: text("access_key_id").notNull(),
  secret_access_key_encrypted: text("secret_access_key_encrypted").notNull(),
  session_token_encrypted: text("session_token_encrypted"),
  is_active: boolean("is_active").notNull().default(true),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  index("s3_providers_provider_type_idx").on(table.provider_type),
  index("s3_providers_is_active_idx").on(table.is_active),
]).enableRLS();

export const organization_s3_bucket = pgTable("organization_s3_buckets", {
  id: uuid("id").primaryKey(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  region_id: uuid("region_id")
    .notNull()
    .references(() => region.id, { onDelete: "cascade" }),
  provider_id: uuid("provider_id")
    .notNull()
    .references(() => s3_provider.id, { onDelete: "cascade" }),
  bucket_name: text("bucket_name").notNull(),
  provider_bucket_name: text("provider_bucket_name").notNull(),
  status: organization_s3_bucket_status("status").notNull().default("active"),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("organization_s3_buckets_org_bucket_uidx").on(table.organization_id, table.bucket_name),
  uniqueIndex("organization_s3_buckets_provider_bucket_uidx").on(table.provider_id, table.provider_bucket_name),
  index("organization_s3_buckets_region_id_idx").on(table.region_id),
  index("organization_s3_buckets_organization_id_idx").on(table.organization_id),
  index("organization_s3_buckets_provider_id_idx").on(table.provider_id),
  pgPolicy("organization_s3_buckets_org_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: sql`${table.organization_id} = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))`,
    withCheck: sql`${table.organization_id} = ANY(COALESCE(current_setting('app.allowed_organizations', true)::uuid[], ARRAY[]::uuid[]))`,
  }),
]).enableRLS();
