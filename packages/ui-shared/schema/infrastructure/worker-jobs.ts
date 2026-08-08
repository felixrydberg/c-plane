import { sql } from "drizzle-orm";
import { check, index, integer, jsonb, pgPolicy, pgTable, text, timestamp, uniqueIndex, uuid } from "drizzle-orm/pg-core";
import { organization } from "../tenants/organization";
import { app_tenant, orgAllowed } from "../rls";

export const worker_job = pgTable("worker_job", {
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
  check("worker_job_status_check", sql`${table.status} in ('queued', 'running', 'succeeded', 'failed')`),
  check("worker_job_attempts_check", sql`${table.attempts} >= 0 and ${table.max_attempts} > 0`),
  index("worker_job_organization_id_idx").on(table.organization_id),
  index("worker_job_claim_idx").on(table.queue_name, table.status, table.available_at, table.created_at),
  index("worker_job_lease_idx").on(table.status, table.lease_expires_at),
  uniqueIndex("worker_job_active_dedupe_uidx")
    .on(table.queue_name, table.dedupe_key)
    .where(sql`${table.dedupe_key} is not null and ${table.status} in ('queued', 'running')`),
  pgPolicy("worker_job_external_registry_cleanup_insert_rls", {
    as: "permissive",
    for: "insert",
    to: app_tenant,
    withCheck: sql`${orgAllowed(table.organization_id)} and ${table.queue_name} = 'secrets' and ${table.job_type} = 'external_registry_secret_cleanup'`,
  }),
]).enableRLS();

export const registry_maintenance = pgTable("registry_maintenance", {
  service: text("service").primaryKey().default("distribution"),
  gc_access_key_id: text("gc_access_key_id").notNull().unique(),
  phase: text("phase").notNull().default("idle"),
  active_job_id: uuid("active_job_id").references(() => worker_job.id, { onDelete: "set null" }),
  started_at: timestamp("started_at", { withTimezone: true, mode: "string" }),
  finished_at: timestamp("finished_at", { withTimezone: true, mode: "string" }),
  last_result: text("last_result"),
  last_error: text("last_error"),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  check("registry_maintenance_phase_check", sql`${table.phase} in ('idle', 'queued', 'draining', 'collecting', 'restoring')`),
  index("registry_maintenance_active_job_idx").on(table.active_job_id),
]).enableRLS();
