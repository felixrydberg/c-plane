import { index, integer, pgEnum, pgTable, text, timestamp, uuid } from "drizzle-orm/pg-core";
import { region } from "../regions/schema";

export const cluster_status = pgEnum("cluster_status", ["active", "inactive", "draining", "offline"]);
export const cluster_health_status = pgEnum("cluster_health_status", ["healthy", "degraded", "offline"]);

export const cluster = pgTable("clusters", {
  id: uuid("id").primaryKey(),
  region_id: uuid("region_id")
    .notNull()
    .references(() => region.id, { onDelete: "cascade" }),
  slug: text("slug").notNull().unique(),
  name: text("name").notNull(),
  agent_id: text("agent_id").notNull().unique(),
  agent_endpoint: text("agent_endpoint").notNull(),
  status: cluster_status("status").notNull().default("active"),
  capacity_allocatable: integer("capacity_allocatable").notNull().default(0),
  capacity_used: integer("capacity_used").notNull().default(0),
  health_status: cluster_health_status("health_status").notNull().default("healthy"),
  agent_last_seen_at: timestamp("agent_last_seen_at", { withTimezone: true, mode: "string" }),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  index("clusters_region_id_idx").on(table.region_id),
  index("clusters_agent_id_idx").on(table.agent_id),
  index("clusters_slug_idx").on(table.slug),
  index("clusters_status_idx").on(table.status),
  index("clusters_health_status_idx").on(table.health_status),
]);
