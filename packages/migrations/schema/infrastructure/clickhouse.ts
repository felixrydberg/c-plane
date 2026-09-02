import { sql } from "drizzle-orm";
import { foreignKey, index, pgPolicy, pgTable, text, timestamp, uniqueIndex, uuid } from "drizzle-orm/pg-core";
import { app_tenant } from "../rls.ts";
import { bucket } from "./buckets.ts";
import { credential, secret } from "./secrets.ts";

export const clickhouse_provider = pgTable.withRLS("clickhouse_providers", {
  id: uuid("id").primaryKey(),
  name: text("name").notNull(),
  endpoint_url: text("endpoint_url").notNull(),
  cluster_name: text("cluster_name").notNull(),
  credential_secret_id: uuid("credential_secret_id").notNull(),
  bucket_id: uuid("bucket_id").notNull(),
  storage_credential_id: uuid("storage_credential_id").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  index("clickhouse_providers_name_idx").on(table.name),
  uniqueIndex("clickhouse_providers_credential_secret_id_uidx").on(table.credential_secret_id),
  uniqueIndex("clickhouse_providers_bucket_id_uidx").on(table.bucket_id),
  uniqueIndex("clickhouse_providers_storage_credential_id_uidx").on(table.storage_credential_id),
  foreignKey({
    columns: [table.credential_secret_id],
    foreignColumns: [secret.id],
    name: "clickhouse_providers_credential_secret_id_fk",
  }).onDelete("restrict"),
  foreignKey({
    columns: [table.bucket_id],
    foreignColumns: [bucket.id],
    name: "clickhouse_providers_bucket_id_fk",
  }).onDelete("restrict"),
  foreignKey({
    columns: [table.storage_credential_id],
    foreignColumns: [credential.id],
    name: "clickhouse_providers_storage_credential_id_fk",
  }).onDelete("restrict"),
  pgPolicy("clickhouse_providers_tenant_select_rls", {
    as: "permissive",
    for: "select",
    to: app_tenant,
    using: sql`true`,
  }),
]);
