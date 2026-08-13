import { index, pgEnum, pgPolicy, pgTable, text, timestamp, uniqueIndex, uuid, boolean } from "drizzle-orm/pg-core";
import { sql } from "drizzle-orm";
import { project } from ".";
import { organization } from "../tenants/organization";
import { app_tenant, orgAllowed } from "../rls";
import { region } from "../infrastructure/regions";

export const bucket_status = pgEnum("bucket_status", ["provisioning", "ready", "deleting", "failed"]);

export const bucket = pgTable('bucket', {
  id: uuid("id").primaryKey(),
  project_id: uuid("project_id")
    .notNull()
    .references(() => project.id, { onDelete: "cascade" }),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  region: uuid("region").notNull().references(() => region.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  status: bucket_status("status").notNull().default("provisioning"),
}, (table) => [
  uniqueIndex("bucket_name_idx").on(table.name),
  index("bucket_project_id_idx").on(table.project_id),
  index("bucket_organization_id_idx").on(table.organization_id),
  index("bucket_region_idx").on(table.region),
  pgPolicy("bucket_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();

export const storage_access_token = pgTable("storage_access_token", {
  id: uuid("id").primaryKey(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  project_id: uuid("project_id")
    .notNull()
    .references(() => project.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  access_key_id: text("access_key_id").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  revoked_at: timestamp("revoked_at", { withTimezone: true, mode: "string" }),
}, (table) => [
  uniqueIndex("storage_access_token_access_key_id_uidx").on(table.access_key_id),
  uniqueIndex("storage_access_token_project_name_uidx")
    .on(table.project_id, table.name)
    .where(sql`${table.revoked_at} is null`),
  index("storage_access_token_organization_id_idx").on(table.organization_id),
  index("storage_access_token_project_id_idx").on(table.project_id),
  pgPolicy("storage_access_token_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();

export const storage_access_token_bucket = pgTable("storage_access_token_bucket", {
  access_token_id: uuid("access_token_id")
    .notNull()
    .references(() => storage_access_token.id, { onDelete: "cascade" }),
  bucket_id: uuid("bucket_id")
    .notNull()
    .references(() => bucket.id, { onDelete: "cascade" }),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  can_read: boolean("can_read").notNull().default(false),
  can_write: boolean("can_write").notNull().default(false),
}, (table) => [
  uniqueIndex("storage_access_token_bucket_uidx").on(table.access_token_id, table.bucket_id),
  index("storage_access_token_bucket_token_id_idx").on(table.access_token_id),
  index("storage_access_token_bucket_bucket_id_idx").on(table.bucket_id),
  pgPolicy("storage_access_token_bucket_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();
