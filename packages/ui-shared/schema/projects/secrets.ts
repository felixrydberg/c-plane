import { pgTable, text, uuid, timestamp, integer, index, uniqueIndex, pgPolicy } from 'drizzle-orm/pg-core';
import { project_branch } from '.';
import { organization } from '../tenants/organization';
import { app_tenant, orgAllowed } from '../rls';

export const secret = pgTable('project_secret', {
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
  uniqueIndex("project_secret_branch_id_name_uidx").on(table.branch_id, table.name),
  index("project_secret_branch_id_idx").on(table.branch_id),
  index("project_secret_organization_id_idx").on(table.organization_id),
  pgPolicy("project_secret_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);

export const secret_version = pgTable('project_secret_version', {
  id: uuid("id").primaryKey(),
  secret_id: uuid("secret_id")
    .notNull()
    .references(() => secret.id, { onDelete: "cascade" }),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  version: integer("version").notNull(),
  value_encrypted: text("value_encrypted").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("project_secret_version_secret_id_version_uidx")
    .on(table.secret_id, table.version),
  index("project_secret_version_secret_id_idx").on(table.secret_id),
  index("project_secret_version_organization_id_idx").on(table.organization_id),
  pgPolicy("project_secret_version_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);
