import { sql } from "drizzle-orm";
import type { AnyPgColumn } from "drizzle-orm/pg-core";
import { boolean, check, foreignKey, index, pgPolicy, pgTable, text, timestamp, uniqueIndex, uuid } from "drizzle-orm/pg-core";
import { app_tenant } from "../rls.ts";
import { secret } from "./secrets.ts";

export const s3_provider = pgTable.withRLS("s3_providers", {
  id: uuid("id").primaryKey(),
  name: text("name").notNull(),
  endpoint_url: text("endpoint_url").notNull(),
  provider_region: text("provider_region").notNull(),
  credential_secret_id: uuid("credential_secret_id").notNull(),
  mirror_provider_id: uuid("mirror_provider_id"),
  is_active: boolean("is_active").notNull().default(true),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  index("s3_providers_name_idx").on(table.name),
  index("s3_providers_is_active_idx").on(table.is_active),
  uniqueIndex("s3_providers_credential_secret_id_uidx").on(table.credential_secret_id),
  index("s3_providers_mirror_provider_id_idx").on(table.mirror_provider_id),
  foreignKey({
    columns: [table.credential_secret_id],
    foreignColumns: [secret.id],
    name: "s3_providers_credential_secret_id_fk",
  }).onDelete("restrict"),
  foreignKey({
    columns: [table.mirror_provider_id],
    foreignColumns: [table.id as AnyPgColumn],
    name: "s3_providers_mirror_provider_id_fk",
  }).onDelete("restrict"),
  check("s3_providers_mirror_not_self_check", sql`${table.mirror_provider_id} is null or ${table.mirror_provider_id} <> ${table.id}`),
  pgPolicy("s3_providers_tenant_select_rls", {
    as: "permissive",
    for: "select",
    to: app_tenant,
    using: sql`true`,
  }),
]);
