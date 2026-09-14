import { pgTable, text, uuid, timestamp, integer, jsonb, boolean, index, uniqueIndex, unique, pgPolicy, foreignKey } from 'drizzle-orm/pg-core';
import type { AnyPgColumn } from 'drizzle-orm/pg-core';
import { sql } from 'drizzle-orm';
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
  draft_timeline: uuid("draft_timeline").notNull(),
  deployed_timeline: uuid("deployed_timeline").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("project_environment_project_id_name_uidx").on(table.project_id, table.name),
  uniqueIndex("project_environment_id_project_id_organization_id_uidx").on(table.id, table.project_id, table.organization_id),
  index("project_environment_organization_id_idx").on(table.organization_id),
  index("project_environment_project_id_idx").on(table.project_id),
  foreignKey({
    columns: [table.draft_timeline, table.project_id, table.organization_id],
    foreignColumns: [project_timeline.id, project_timeline.project_id, project_timeline.organization_id],
    name: "project_environment_draft_scope_fk",
  }).onDelete("no action"),
  foreignKey({
    columns: [table.deployed_timeline, table.project_id, table.organization_id],
    foreignColumns: [project_timeline.id, project_timeline.project_id, project_timeline.organization_id],
    name: "project_environment_deployed_scope_fk",
  }).onDelete("no action"),
  pgPolicy("project_environment_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);

export const project_revision_manifest = pgTable.withRLS('project_revision_manifest', {
  id: uuid("id").primaryKey(),
  project_id: uuid("project_id")
    .notNull()
    .references(() => project.id, { onDelete: "cascade" }),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  schema_version: integer("schema_version").notNull(),
  configuration: jsonb("configuration").notNull(),
  external_registry_ids: uuid("external_registry_ids").array().notNull().default(sql`'{}'::uuid[]`),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  unique("project_revision_manifest_scope_uidx").on(
    table.id,
    table.project_id,
    table.organization_id,
  ),
  index("project_revision_manifest_project_id_idx").on(table.project_id),
  index("project_revision_manifest_organization_id_idx").on(table.organization_id),
  index("project_revision_manifest_external_registry_ids_idx").using("gin", table.external_registry_ids),
  pgPolicy("project_revision_manifest_select", {
    as: "permissive",
    for: "select",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
  }),
  pgPolicy("project_revision_manifest_insert", {
    as: "permissive",
    for: "insert",
    to: app_tenant,
    withCheck: orgAllowed(table.organization_id),
  }),
  pgPolicy("project_revision_manifest_delete", {
    as: "permissive",
    for: "delete",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
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
  manifest_id: uuid("manifest_id").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  unique("project_timeline_parent_scope_uidx").on(
    table.id,
    table.project_id,
    table.organization_id,
  ),
  unique("project_timeline_project_id_timeline_uidx").on(table.project_id, table.timeline),
  foreignKey({
    columns: [table.parent_timeline_id, table.project_id, table.organization_id],
    foreignColumns: [table.id, table.project_id, table.organization_id],
    name: "project_timeline_parent_scope_fk",
  }).onDelete("no action"),
  foreignKey({
    columns: [table.manifest_id, table.project_id, table.organization_id],
    foreignColumns: [project_revision_manifest.id, project_revision_manifest.project_id, project_revision_manifest.organization_id],
    name: "project_timeline_manifest_scope_fk",
  }).onDelete("no action"),
  index("project_timeline_id_idx").on(table.id),
  index("project_timeline_environment_id_idx").on(table.environment_id),
  index("project_timeline_organization_id_idx").on(table.organization_id),
  index("project_timeline_project_id_idx").on(table.project_id),
  index("project_timeline_parent_timeline_id_idx").on(table.parent_timeline_id),
  index("project_timeline_manifest_id_idx").on(table.manifest_id),
  index("project_timeline_project_id_environment_id_timeline_idx").on(table.project_id, table.environment_id, table.timeline.desc()),
  pgPolicy("project_timeline_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);
