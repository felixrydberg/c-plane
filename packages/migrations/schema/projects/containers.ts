import { pgTable, text, uuid, timestamp, index, pgPolicy } from 'drizzle-orm/pg-core';
import { project } from './index.ts';
import { organization } from '../tenants/organization.ts';
import { region } from '../infrastructure/regions.ts';
import { app_tenant, orgAllowed } from '../rls.ts';

export const container = pgTable.withRLS('container', {
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
  index("container_organization_id_idx").on(table.organization_id),
  index("container_project_id_idx").on(table.project_id),
  pgPolicy("container_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);
