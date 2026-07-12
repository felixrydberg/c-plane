import { index, pgPolicy, pgTable, text, uuid, boolean } from "drizzle-orm/pg-core";
import { project } from ".";
import { organization } from "../tenants/organization";
import { app_tenant, orgAllowed } from "../rls";
import { region } from "../infrastructure/regions";

export const bucket = pgTable('bucket', {
  id: uuid("id").primaryKey(),
  project_id: uuid("project_id")
    .notNull()
    .references(() => project.id, { onDelete: "cascade" }),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  region: uuid("region").notNull().references(() => region.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  is_public: boolean("is_public").notNull().default(false),
}, (table) => [
  index("bucket_project_id_idx").on(table.project_id),
  index("bucket_organization_id_idx").on(table.organization_id),
  index("bucket_region_idx").on(table.region),
  pgPolicy("bucket_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();
