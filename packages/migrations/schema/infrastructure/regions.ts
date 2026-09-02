import { sql } from "drizzle-orm";
import { foreignKey, index, pgEnum, pgPolicy, pgTable, text, timestamp, uuid } from "drizzle-orm/pg-core";
import { clickhouse_provider } from "./clickhouse.ts";
import { s3_provider } from "./durability.ts";
import { app_tenant } from "../rls.ts";

export const region_status = pgEnum("region_status", ["active", "inactive", "maintenance"]);
export const region_routing_mode = pgEnum("region_routing_mode", ["active", "draining", "disabled"]);

export const region = pgTable.withRLS("regions", {
  id: uuid("id").primaryKey(),
  slug: text("slug").notNull().unique(),
  display_name: text("display_name").notNull(),
  s3_provider_id: uuid("s3_provider_id").notNull().references(() => s3_provider.id, { onDelete: "restrict" }),
  clickhouse_provider_id: uuid("clickhouse_provider_id"),
  status: region_status("status").notNull().default("active"),
  routing_mode: region_routing_mode("routing_mode").notNull().default("active"),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  index("regions_slug_idx").on(table.slug),
  index("regions_status_idx").on(table.status),
  index("regions_routing_mode_idx").on(table.routing_mode),
  index("regions_s3_provider_id_idx").on(table.s3_provider_id),
  index("regions_clickhouse_provider_id_idx").on(table.clickhouse_provider_id),
  foreignKey({
    columns: [table.clickhouse_provider_id],
    foreignColumns: [clickhouse_provider.id],
    name: "regions_clickhouse_provider_id_fk",
  }).onDelete("restrict"),
  pgPolicy("regions_tenant_select_rls", {
    as: "permissive",
    for: "select",
    to: app_tenant,
    using: sql`true`,
  }),
]);
