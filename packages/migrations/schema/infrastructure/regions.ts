import { sql } from "drizzle-orm";
import { index, pgEnum, pgPolicy, pgTable, text, uniqueIndex, timestamp, uuid } from "drizzle-orm/pg-core";
import { s3_provider } from "./durability.ts";
import { app_tenant } from "../rls.ts";

export const region_status = pgEnum("region_status", ["active", "inactive", "maintenance"]);
export const region_routing_mode = pgEnum("region_routing_mode", ["active", "draining", "disabled"]);

export const region = pgTable("regions", {
  id: uuid("id").primaryKey(),
  slug: text("slug").notNull().unique(),
  display_name: text("display_name").notNull(),
  s3_provider_id: uuid("s3_provider_id").references(() => s3_provider.id, { onDelete: "set null" }),
  status: region_status("status").notNull().default("active"),
  routing_mode: region_routing_mode("routing_mode").notNull().default("active"),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  index("regions_slug_idx").on(table.slug),
  index("regions_status_idx").on(table.status),
  index("regions_routing_mode_idx").on(table.routing_mode),
  index("regions_s3_provider_id_idx").on(table.s3_provider_id),
  pgPolicy("regions_tenant_select_rls", {
    as: "permissive",
    for: "select",
    to: app_tenant,
    using: sql`true`,
  }),
]).enableRLS();
