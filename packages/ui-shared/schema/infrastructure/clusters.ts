import { boolean, index, integer, pgEnum, pgPolicy, pgTable, text, timestamp, uniqueIndex, uuid } from "drizzle-orm/pg-core";
import { region, region_capability } from "./regions";
import { cplane_user } from "../tenants/control-plane-ui";
import { app_tenant } from "../rls";
import { sql } from "drizzle-orm";

export const cluster_status = pgEnum("cluster_status", ["pending", "bootstrapping", "healthy", "draining", "offline", "removed"]);
export const cluster_health_status = pgEnum("cluster_health_status", ["healthy", "degraded", "offline"]);
export const cluster_provider = pgEnum("cluster_provider", ["aws", "gcp", "azure", "metal"]);
export const cluster_ingress_endpoint_health_status = pgEnum("cluster_ingress_endpoint_health_status", ["healthy", "degraded", "unreachable"]);

export const cluster = pgTable("clusters", {
  id: uuid("id").primaryKey(),
  region_id: uuid("region_id")
    .notNull()
    .references(() => region.id, { onDelete: "cascade" }),
  slug: text("slug").notNull().unique(),
  name: text("name").notNull(),
  agent_id: text("agent_id").unique(),
  agent_endpoint: text("agent_endpoint"),
  status: cluster_status("status").notNull().default("pending"),
  provider: cluster_provider("provider").notNull().default("aws"),
  capacity_allocatable: integer("capacity_allocatable").notNull().default(0),
  capacity_used: integer("capacity_used").notNull().default(0),
  health_status: cluster_health_status("health_status").notNull().default("healthy"),
  agent_last_seen_at: timestamp("agent_last_seen_at", { withTimezone: true, mode: "string" }),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  index("clusters_region_id_idx").on(table.region_id),
  index("clusters_agent_id_idx").on(table.agent_id),
  index("clusters_slug_idx").on(table.slug),
  index("clusters_status_idx").on(table.status),
  index("clusters_health_status_idx").on(table.health_status),
  index("clusters_provider_idx").on(table.provider),
]).enableRLS();

export const cluster_capabilities = pgTable("cluster_capabilities", {
  id: uuid("id").primaryKey(),
  cluster_id: uuid("cluster_id")
    .notNull()
    .references(() => cluster.id, { onDelete: "cascade" }),
  capability: region_capability("capability").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("cluster_capabilities_cluster_id_capability_uidx").on(table.cluster_id, table.capability),
  index("cluster_capabilities_capability_idx").on(table.capability),
  pgPolicy("cluster_capabilities_tenant_select_rls", {
    as: "permissive",
    for: "select",
    to: app_tenant,
    using: sql`true`,
  }),
]).enableRLS();

export const cluster_ingress_endpoint = pgTable("cluster_ingress_endpoints", {
  id: uuid("id").primaryKey(),
  cluster_id: uuid("cluster_id")
    .notNull()
    .references(() => cluster.id, { onDelete: "cascade" }),
  address: text("address").notNull(),
  port: integer("port").notNull().default(443),
  enabled: boolean("enabled").notNull().default(true),
  health_status: cluster_ingress_endpoint_health_status("health_status").notNull().default("healthy"),
  last_seen_at: timestamp("last_seen_at", { withTimezone: true, mode: "string" }),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("cluster_ingress_endpoints_cluster_id_address_uidx").on(table.cluster_id, table.address),
  index("cluster_ingress_endpoints_cluster_id_idx").on(table.cluster_id),
  index("cluster_ingress_endpoints_health_status_idx").on(table.health_status),
  index("cluster_ingress_endpoints_enabled_idx").on(table.enabled),
]).enableRLS();

export const cluster_join_credential = pgTable("cluster_join_credentials", {
  id: uuid("id").primaryKey(),
  cluster_id: uuid("cluster_id")
    .notNull()
    .references(() => cluster.id, { onDelete: "cascade" }),
  token_hash: text("token_hash").notNull(),
  expires_at: timestamp("expires_at", { withTimezone: true, mode: "string" }).notNull(),
  used_at: timestamp("used_at", { withTimezone: true, mode: "string" }),
  revoked_at: timestamp("revoked_at", { withTimezone: true, mode: "string" }),
  revoked_reason: text("revoked_reason"),
  issued_by_user_id: uuid("issued_by_user_id")
    .references(() => cplane_user.id, { onDelete: "set null" }),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  index("cluster_join_credentials_cluster_id_idx").on(table.cluster_id),
  index("cluster_join_credentials_expires_at_idx").on(table.expires_at),
  index("cluster_join_credentials_token_hash_idx").on(table.token_hash),
]).enableRLS();

export const cluster_runtime_identity = pgTable("cluster_runtime_identities", {
  id: uuid("id").primaryKey(),
  cluster_id: uuid("cluster_id")
    .notNull()
    .references(() => cluster.id, { onDelete: "cascade" }),
  public_key_pem: text("public_key_pem").notNull(),
  key_algorithm: text("key_algorithm").notNull().default("ed25519"),
  key_version: integer("key_version").notNull().default(1),
  lease_epoch: integer("lease_epoch").notNull().default(0),
  last_rotated_at: timestamp("last_rotated_at", { withTimezone: true, mode: "string" }),
  last_seen_at: timestamp("last_seen_at", { withTimezone: true, mode: "string" }),
  revoked_at: timestamp("revoked_at", { withTimezone: true, mode: "string" }),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("cluster_runtime_identities_cluster_id_uidx").on(table.cluster_id),
  index("cluster_runtime_identities_revoked_at_idx").on(table.revoked_at),
]).enableRLS();
