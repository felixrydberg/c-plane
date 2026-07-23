import { pgTable, text, uuid, timestamp, integer, boolean, index, uniqueIndex, jsonb, pgPolicy } from 'drizzle-orm/pg-core';
import { project } from '.';
import { organization } from '../tenants/organization';
import { region } from '../infrastructure/regions';
import { app_tenant, orgAllowed } from '../rls';

export const container = pgTable('project_container', {
  id: uuid("id").primaryKey(),
  project_id: uuid("project_id")
    .notNull()
    .references(() => project.id, { onDelete: "cascade" }),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  region_id: uuid("region_id")
    .notNull()
    .references(() => region.id, { onDelete: "restrict" }),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  index("project_container_organization_id_idx").on(table.organization_id),
  index("project_container_project_id_idx").on(table.project_id),
  pgPolicy("project_container_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();

export const container_version = pgTable('project_container_version', {
  id: uuid("id").primaryKey(),
  container_id: uuid("container_id")
    .notNull()
    .references(() => container.id, { onDelete: "cascade" }),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  version: integer("version").notNull(),
  image: text("image").notNull(),
  public: boolean("public").notNull().default(false),
  replica_count: integer("replica_count").notNull().default(1),
  port: integer("port"),
  env: jsonb("env"),
  env_secret_refs: jsonb("env_secret_refs"),
  resources: jsonb("resources"),
  pull_secret_id: uuid("pull_secret_id"),
  health_check: jsonb("health_check"),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("project_container_version_container_id_version_uidx")
    .on(table.container_id, table.version),
  index("project_container_version_container_id_idx").on(table.container_id),
  index("project_container_version_organization_id_idx").on(table.organization_id),
  pgPolicy("project_container_version_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();
