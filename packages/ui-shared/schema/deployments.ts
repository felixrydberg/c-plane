import { organization } from "./organization";
import { region } from "./regions";
import { pgTable, integer, uuid, pgEnum, text, bigserial, boolean, index, pgPolicy } from "drizzle-orm/pg-core";
import { app_tenant, orgAllowed, deploymentAllowed } from "./rls";

export const deployment_types = ["container", "cnpg", "redis"] as const;
export const deployment_type = pgEnum("deployment_type", deployment_types);

export const deployment = pgTable("deployments", {
  id: uuid("id").primaryKey(),
  name: text("name").notNull(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  type: deployment_type("type").notNull(),
  created_at: text("created_at").notNull(),
  updated_at: text("updated_at").notNull(),
}, (table) => [
  pgPolicy("deployments_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();

export const deployment_region = pgTable("deployment_regions", {
  id: bigserial("id", { mode: "number" }).primaryKey(),
  deployment_id: uuid("deployment_id")
    .notNull()
    .references(() => deployment.id, { onDelete: "cascade" }),
  region_id: uuid("region_id")
    .notNull()
    .references(() => region.id, { onDelete: "cascade" }),
}, (table) => [
  pgPolicy("deployment_regions_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: deploymentAllowed(table.deployment_id),
    withCheck: deploymentAllowed(table.deployment_id),
  }),
]).enableRLS();

export const deployment_containers = pgTable("deployment_containers", {
  id: bigserial("id", { mode: "number" }).primaryKey(),
  deployment_id: uuid("deployment_id")
    .notNull()
    .references(() => deployment.id, { onDelete: "cascade" }),
  port: integer("port").notNull().default(80),
  replicas: integer("replicas").notNull().default(1),
  public: boolean("public").notNull().default(false),
  image: text("image").notNull(),
  health_check_path: text("health_check_path").notNull().default("/health"),
}, (table) => [
  index("deployment_containers_deployment_id_idx").on(table.deployment_id),
  pgPolicy("deployment_containers_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: deploymentAllowed(table.deployment_id),
    withCheck: deploymentAllowed(table.deployment_id),
  }),
]).enableRLS();

// export const deployment_cnpg = pgTable("deployment_cnpg", {
//   id: bigserial("id", { mode: "number" }).primaryKey(),
//   deployment_id: uuid("deployment_id")
//     .notNull()
//     .references(() => deployment.id, { onDelete: "cascade" }),
// }, (table) => [
//   pgPolicy("deployment_cnpg_tenant_rls", {
//     as: "permissive",
//     for: "all",
//     to: app_tenant,
//     using: deploymentAllowed(table.deployment_id),
//     withCheck: deploymentAllowed(table.deployment_id),
//   }),
// ]).enableRLS();
