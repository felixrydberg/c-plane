import { pgTable, text, uuid, timestamp, integer, boolean, index, uniqueIndex, jsonb } from 'drizzle-orm/pg-core';
import { project_branch } from '.';
import { organization } from '../tenants/organization';

export const container = pgTable('project_container', {
  id: uuid("id").primaryKey(),
  branch_id: uuid("branch_id")
    .notNull()
    .references(() => project_branch.id, { onDelete: "cascade" }),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("project_container_branch_id_name_uidx").on(table.branch_id, table.name),
  index("project_container_branch_id_idx").on(table.branch_id),
  index("project_container_organization_id_idx").on(table.organization_id),
]);

export const container_version = pgTable('project_container_version', {
  id: uuid("id").primaryKey(),
  container_id: uuid("container_id")
    .notNull()
    .references(() => container.id, { onDelete: "cascade" }),
  version: integer("version").notNull(),
  image: text("image").notNull(),
  public: boolean("public").notNull().default(false),
  replica_count: integer("replica_count").notNull().default(1),
  port: integer("port"),
  env: jsonb("env"),
  resources: jsonb("resources"),
  pull_secret_id: uuid("pull_secret_id"),
  health_check: jsonb("health_check"),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("project_container_version_container_id_version_uidx")
    .on(table.container_id, table.version),
  index("project_container_version_container_id_idx").on(table.container_id),
]);
