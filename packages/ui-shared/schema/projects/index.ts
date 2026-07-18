import { pgTable, text, uuid, timestamp, integer, jsonb, index, uniqueIndex, unique, pgPolicy, foreignKey } from 'drizzle-orm/pg-core';
import type { AnyPgColumn } from 'drizzle-orm/pg-core';
import { organization } from '../tenants/organization';
import { app_tenant, orgAllowed } from '../rls';
export * from './containers';
export * from './stateful_postgres';
export * from './storage';

export const project = pgTable('project', {
  id: uuid("id").primaryKey(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  default_branch_id: uuid("default_branch_id")
    .references((): AnyPgColumn => project_branch.id, { onDelete: "set null" }),
  name: text("name").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("project_organization_id_name_uidx").on(table.organization_id, table.name),
  uniqueIndex("project_id_organization_id_uidx").on(table.id, table.organization_id),
  index("project_organization_id_idx").on(table.organization_id),
  index("project_default_branch_id_idx").on(table.default_branch_id),
  pgPolicy("project_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();

export const project_branch = pgTable('project_branch', {
  id: uuid("id").primaryKey(),
  project_id: uuid("project_id")
    .notNull()
    .references(() => project.id, { onDelete: "cascade" }),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  timeline: uuid("timeline").notNull().references(() => project_timeline.id, { onDelete: "restrict" }),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("project_branch_project_id_name_uidx").on(table.project_id, table.name),
  uniqueIndex("project_branch_id_project_id_organization_id_uidx").on(table.id, table.project_id, table.organization_id),
  index("project_branch_organization_id_idx").on(table.organization_id),
  index("project_branch_project_id_idx").on(table.project_id),
  pgPolicy("project_branch_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();

export const project_timeline = pgTable('project_timeline', {
  id: uuid("id").primaryKey(),
  project_id: uuid("project_id")
    .notNull()
    .references(() => project.id, { onDelete: "cascade" }),
  branch_id: uuid("branch_id"),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  timeline: integer("timeline").notNull(),
  name: text("name"),
  parent_timeline_id: uuid("parent_timeline_id")
    .references((): AnyPgColumn => project_timeline.id, { onDelete: "set null" }),
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
  }),
  index("project_timeline_id_idx").on(table.id),
  index("project_timeline_branch_id_idx").on(table.branch_id),
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
]).enableRLS();
