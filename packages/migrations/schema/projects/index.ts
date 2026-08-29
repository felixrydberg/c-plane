import { pgTable, text, uuid, timestamp, integer, jsonb, boolean, index, uniqueIndex, unique, pgPolicy, foreignKey } from 'drizzle-orm/pg-core';
import type { AnyPgColumn } from 'drizzle-orm/pg-core';
import { organization } from '../tenants/organization.ts';
import { app_tenant, orgAllowed } from '../rls.ts';
export * from './containers.ts';
export * from './postgres.ts';
export * from './storage.ts';

export const project = pgTable.withRLS('project', {
  id: uuid("id").primaryKey(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  default_environment_id: uuid("default_environment_id")
    .references((): AnyPgColumn => project_environment.id, { onDelete: "set null" }),
  name: text("name").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("project_organization_id_name_uidx").on(table.organization_id, table.name),
  uniqueIndex("project_id_organization_id_uidx").on(table.id, table.organization_id),
  index("project_organization_id_idx").on(table.organization_id),
  index("project_default_environment_id_idx").on(table.default_environment_id),
  pgPolicy("project_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);

export const project_environment = pgTable.withRLS('project_environment', {
  id: uuid("id").primaryKey(),
  project_id: uuid("project_id")
    .notNull()
    .references(() => project.id, { onDelete: "cascade" }),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  is_preview: boolean("is_preview").notNull().default(true),
  draft_timeline: uuid("draft_timeline").notNull().references(() => project_timeline.id, { onDelete: "restrict" }),
  deployed_timeline: uuid("deployed_timeline").notNull().references(() => project_timeline.id, { onDelete: "restrict" }),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("project_environment_project_id_name_uidx").on(table.project_id, table.name),
  uniqueIndex("project_environment_id_project_id_organization_id_uidx").on(table.id, table.project_id, table.organization_id),
  index("project_environment_organization_id_idx").on(table.organization_id),
  index("project_environment_project_id_idx").on(table.project_id),
  pgPolicy("project_environment_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);

export const project_timeline = pgTable.withRLS('project_timeline', {
  id: uuid("id").primaryKey(),
  project_id: uuid("project_id")
    .notNull()
    .references(() => project.id, { onDelete: "cascade" }),
  environment_id: uuid("environment_id"),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  timeline: integer("timeline").notNull(),
  name: text("name"),
  parent_timeline_id: uuid("parent_timeline_id"),
  pins: jsonb("pins").notNull().default({}),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  unique("project_timeline_parent_scope_uidx").on(
    table.id,
    table.project_id,
    table.organization_id,
  ),
  foreignKey({
    columns: [table.parent_timeline_id, table.project_id, table.organization_id],
    foreignColumns: [table.id, table.project_id, table.organization_id],
    name: "project_timeline_parent_scope_fk",
  }).onDelete("no action"),
  index("project_timeline_id_idx").on(table.id),
  index("project_timeline_environment_id_idx").on(table.environment_id),
  index("project_timeline_organization_id_idx").on(table.organization_id),
  index("project_timeline_project_id_idx").on(table.project_id),
  index("project_timeline_parent_timeline_id_idx").on(table.parent_timeline_id),
  pgPolicy("project_timeline_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);
