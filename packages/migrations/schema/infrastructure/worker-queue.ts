import { sql } from "drizzle-orm";
import { check, index, integer, jsonb, pgPolicy, pgTable, text, timestamp, uniqueIndex, uuid } from "drizzle-orm/pg-core";
import { organization } from "../tenants/organization.ts";
import { app_tenant, orgAllowed } from "../rls.ts";

export const worker_queue = pgTable.withRLS("worker_queue", {
  id: uuid("id").primaryKey(),
  organization_id: uuid("organization_id").references(() => organization.id, { onDelete: "cascade" }),
  queue_name: text("queue_name").notNull(),
  job_type: text("job_type").notNull(),
  payload: jsonb("payload").notNull().default({}),
  status: text("status").notNull().default("queued"),
  dedupe_key: text("dedupe_key"),
  attempts: integer("attempts").notNull().default(0),
  max_attempts: integer("max_attempts").notNull().default(3),
  available_at: timestamp("available_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  locked_by: text("locked_by"),
  lease_expires_at: timestamp("lease_expires_at", { withTimezone: true, mode: "string" }),
  last_error: text("last_error"),
  started_at: timestamp("started_at", { withTimezone: true, mode: "string" }),
  finished_at: timestamp("finished_at", { withTimezone: true, mode: "string" }),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  check("worker_queue_status_check", sql`${table.status} in ('queued', 'running', 'succeeded', 'failed')`),
  check("worker_queue_attempts_check", sql`${table.attempts} >= 0 and ${table.max_attempts} > 0`),
  index("worker_queue_organization_id_idx").on(table.organization_id),
  index("worker_queue_claim_idx").on(table.queue_name, table.status, table.available_at, table.created_at),
  index("worker_queue_lease_idx").on(table.status, table.lease_expires_at),
  uniqueIndex("worker_queue_active_dedupe_uidx")
    .on(table.queue_name, table.dedupe_key)
    .where(sql`${table.dedupe_key} is not null and ${table.status} in ('queued', 'running')`),
  pgPolicy("worker_queue_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);
