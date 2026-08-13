import { sql } from "drizzle-orm";
import { index, pgPolicy, pgTable, text, timestamp, uniqueIndex, uuid } from "drizzle-orm/pg-core";
import { s3_provider } from "./durability";
import { app_tenant } from "../rls";

export const registry_storage = pgTable("registry_storage", {
  id: uuid("id").primaryKey(),
  service: text("service").notNull().default("distribution"),
  provider_id: uuid("provider_id")
    .notNull()
    .references(() => s3_provider.id, { onDelete: "restrict" }),
  bucket_name: text("bucket_name").notNull(),
  physical_bucket_name: text("physical_bucket_name").notNull(),
  access_key_id: text("access_key_id").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("registry_storage_service_uidx").on(table.service),
  uniqueIndex("registry_storage_bucket_name_uidx").on(table.bucket_name),
  uniqueIndex("registry_storage_physical_bucket_name_uidx").on(table.physical_bucket_name),
  uniqueIndex("registry_storage_access_key_id_uidx").on(table.access_key_id),
  index("registry_storage_provider_id_idx").on(table.provider_id),
  pgPolicy("registry_storage_tenant_select_rls", {
    as: "permissive",
    for: "select",
    to: app_tenant,
    using: sql`true`,
  }),
]).enableRLS();
