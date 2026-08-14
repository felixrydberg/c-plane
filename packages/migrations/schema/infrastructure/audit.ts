import { sql } from "drizzle-orm";
import { index, jsonb, pgPolicy, pgTable, text, timestamp, uuid } from "drizzle-orm/pg-core";
import { app_audit_reader } from "../rls";

export const infrastructure_audit_log = pgTable("infrastructure_audit_log", {
  id: uuid("id").primaryKey(),
  actor_identifier: text("actor_identifier").notNull(),
  source_ip: text("source_ip").notNull(),
  action: text("action").notNull(),
  resource_type: text("resource_type").notNull(),
  resource_id: uuid("resource_id"),
  changes: jsonb("changes").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  index("infrastructure_audit_log_created_at_idx").on(table.created_at),
  index("infrastructure_audit_log_resource_idx").on(table.resource_type, table.resource_id),
  pgPolicy("infrastructure_audit_log_reader", {
    as: "permissive",
    for: "select",
    to: app_audit_reader,
    using: sql`true`,
  }),
]).enableRLS();
