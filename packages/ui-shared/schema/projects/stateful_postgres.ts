import { boolean, pgTable, text, uuid, index, pgPolicy, AnyPgColumn } from "drizzle-orm/pg-core"
import { project, project_branch } from "."
import { app_tenant, orgAllowed } from "../rls";

export const stateful_postgres_database = pgTable('stateful_postgres_database', {
  id: uuid("id").primaryKey(),
  project_id: uuid("project_id")
    .notNull()
    .references(() => project.id, { onDelete: "cascade" }),
  default_branch_id: uuid("default_branch_id")
    .references((): AnyPgColumn => stateful_postgres_database_branch.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  autoscaling_enabled: boolean("autoscaling_enabled").notNull().default(false),
  autoscaling_min_cpu: text("autoscaling_min_cpu"),
  autoscaling_max_cpu: text("autoscaling_max_cpu"),
}, (table) => [
  index("stateful_postgres_database_project_id_idx").on(table.project_id),
  pgPolicy("stateful_postgres_database_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.project_id),
    withCheck: orgAllowed(table.project_id),
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

}, (table) => [
  index("stateful_postgres_database_branch_database_id_idx").on(table.database_id),
  index("stateful_postgres_database_branch_branch_id_idx").on(table.branch_id),
  pgPolicy("stateful_postgres_database_branch_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.branch_id),
    withCheck: orgAllowed(table.branch_id),
  }),
]).enableRLS();
