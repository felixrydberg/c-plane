import { sql } from "drizzle-orm";
import {
  boolean,
  check,
  foreignKey,
  index,
  pgEnum,
  pgPolicy,
  pgTable,
  text,
  timestamp,
  uniqueIndex,
  uuid,
} from "drizzle-orm/pg-core";
import { app_tenant, orgAllowed } from "../rls.ts";
import { region } from "./regions.ts";
import { credential, secret } from "./secrets.ts";

export const FOUNDATION_BUCKET_STATUSES = ["active", "deleting"] as const;
export const foundation_bucket_status = pgEnum("foundation_bucket_status", FOUNDATION_BUCKET_STATUSES);

// Foundation bucket, no RLS: holds tenant buckets and internal/platform buckets alike.
// Tenant scoping happens on storage_bucket / bucket_grant / storage_access_token.
export const bucket = pgTable("bucket", {
  id: uuid("id").primaryKey(),
  region_id: uuid("region_id").notNull().references(() => region.id, { onDelete: "restrict" }),
  sse_secret_id: uuid("sse_secret_id").notNull().references(() => secret.id, { onDelete: "restrict" }),
  status: foundation_bucket_status("status").notNull().default("active"),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("bucket_sse_secret_id_uidx").on(table.sse_secret_id),
  index("bucket_region_id_idx").on(table.region_id),
  index("bucket_status_idx").on(table.status),
]);

export const bucket_grant = pgTable.withRLS("bucket_grant", {
  id: uuid("id").primaryKey(),
  credential_id: uuid("credential_id").notNull(),
  bucket_id: uuid("bucket_id").notNull(),
  organization_id: uuid("organization_id"),
  prefix: text("prefix").notNull().default(""),
  can_read: boolean("can_read").notNull().default(false),
  can_write: boolean("can_write").notNull().default(false),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  check("bucket_grant_permission_check", sql`${table.can_read} or ${table.can_write}`),
  uniqueIndex("bucket_grant_credential_bucket_uidx").on(
    table.credential_id,
    table.bucket_id,
  ),
  foreignKey({
    columns: [table.credential_id],
    foreignColumns: [credential.id],
    name: "bucket_grant_credential_id_fkey",
  }).onDelete("cascade"),
  foreignKey({
    columns: [table.credential_id, table.organization_id],
    foreignColumns: [credential.id, credential.organization_id],
    name: "bucket_grant_credential_id_credential_id_fkey",
  }).onDelete("cascade"),
  foreignKey({
    columns: [table.bucket_id],
    foreignColumns: [bucket.id],
    name: "bucket_grant_bucket_id_fk",
  }).onDelete("restrict"),
  index("bucket_grant_credential_id_idx").on(table.credential_id),
  index("bucket_grant_bucket_id_idx").on(table.bucket_id),
  index("bucket_grant_organization_id_idx").on(table.organization_id),
  pgPolicy("bucket_grant_platform_select_rls", {
    as: "permissive",
    for: "select",
    to: app_tenant,
    using: sql`${table.organization_id} is null`,
  }),
  pgPolicy("bucket_grant_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);
