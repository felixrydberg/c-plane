import { boolean, integer, pgTable, text, uuid, index, pgPolicy, AnyPgColumn } from "drizzle-orm/pg-core"
import { project, project_environment } from "."
import { organization } from "../tenants/organization";
import { app_tenant, orgAllowed } from "../rls";

export const postgres_database = pgTable('postgres_database', {
  id: uuid("id").primaryKey(),
  project_id: uuid("project_id")
    .notNull()
    .references(() => project.id, { onDelete: "cascade" }),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  default_branch_id: uuid("default_branch_id")
    .references((): AnyPgColumn => postgres_database_branch.id, { onDelete: "set null" }),
  name: text("name").notNull(),
}, (table) => [
  index("postgres_database_project_id_idx").on(table.project_id),
  index("postgres_database_organization_id_idx").on(table.organization_id),
  pgPolicy("postgres_database_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();

export const postgres_database_branch = pgTable('postgres_database_branch', {
  id: uuid("id").primaryKey(),
  database_id: uuid("database_id")
    .notNull()
    .references((): AnyPgColumn => postgres_database.id, { onDelete: "cascade" }),
  branch_id: uuid("branch_id")
    .notNull()
    .references(() => project_environment.id, { onDelete: "cascade" }),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  backup_retention_days: integer("backup_retention_days").default(30),
  cpu: text("cpu"),
  ram: text("ram"),
  high_availability: boolean("high_availability").notNull().default(false),
  read_replicas: integer("read_replicas"),
  autoscaling_enabled: boolean("autoscaling_enabled").notNull().default(false),
  autoscaling_min_cpu: text("autoscaling_min_cpu"),
  autoscaling_max_cpu: text("autoscaling_max_cpu"),

}, (table) => [
  index("postgres_database_branch_database_id_idx").on(table.database_id),
  index("postgres_database_branch_branch_id_idx").on(table.branch_id),
  index("postgres_database_branch_organization_id_idx").on(table.organization_id),
  pgPolicy("postgres_database_branch_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();
