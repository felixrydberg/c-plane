import { boolean, integer, pgTable, text, uuid, index, pgPolicy, AnyPgColumn } from "drizzle-orm/pg-core"
import { project, project_branch } from "."
import { organization } from "../tenants/organization";
import { app_tenant, orgAllowed } from "../rls";

export const stateful_postgres_database = pgTable('stateful_postgres_database', {
  id: uuid("id").primaryKey(),
  project_id: uuid("project_id")
    .notNull()
    .references(() => project.id, { onDelete: "cascade" }),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  default_branch_id: uuid("default_branch_id")
    .references((): AnyPgColumn => stateful_postgres_database_branch.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  cpu: text("cpu"),
  ram: text("ram"),
  high_availability: boolean("high_availability").notNull().default(false),
  read_replicas: integer("read_replicas"),
  autoscaling_enabled: boolean("autoscaling_enabled").notNull().default(false),
  autoscaling_min_cpu: text("autoscaling_min_cpu"),
  autoscaling_max_cpu: text("autoscaling_max_cpu"),
}, (table) => [
  index("stateful_postgres_database_project_id_idx").on(table.project_id),
  index("stateful_postgres_database_organization_id_idx").on(table.organization_id),
  pgPolicy("stateful_postgres_database_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();

export const stateful_postgres_database_branch = pgTable('stateful_postgres_database_branch', {
  id: uuid("id").primaryKey(),
  database_id: uuid("database_id")
    .notNull()
    .references((): AnyPgColumn => stateful_postgres_database.id, { onDelete: "cascade" }),
  branch_id: uuid("branch_id")
    .notNull()
    .references(() => project_branch.id, { onDelete: "cascade" }),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),

}, (table) => [
  index("stateful_postgres_database_branch_database_id_idx").on(table.database_id),
  index("stateful_postgres_database_branch_branch_id_idx").on(table.branch_id),
  index("stateful_postgres_database_branch_organization_id_idx").on(table.organization_id),
  pgPolicy("stateful_postgres_database_branch_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();
