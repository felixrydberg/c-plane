import { boolean, foreignKey, index, pgPolicy, pgTable, text, timestamp, unique, uniqueIndex, uuid } from "drizzle-orm/pg-core";
import { sql } from "drizzle-orm";
import { app_tenant, orgAllowed } from "../rls.ts";
import { organization } from "./organization.ts";

export const registry_repositories = pgTable("registry_repositories", {
  id: uuid("id").primaryKey(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  unique("registry_repositories_id_organization_id_uidx").on(table.id, table.organization_id),
  uniqueIndex("registry_repositories_organization_name_uidx").on(table.organization_id, table.name),
  index("registry_repositories_organization_id_idx").on(table.organization_id),
  pgPolicy("registry_repositories_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();

export const external_registry = pgTable("external_registry", {
  id: uuid("id").primaryKey(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  host: text("host").notNull(),
  username: text("username").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  unique("external_registry_id_organization_id_uidx").on(table.id, table.organization_id),
  uniqueIndex("external_registry_organization_name_uidx").on(table.organization_id, table.name),
  uniqueIndex("external_registry_organization_host_username_uidx").on(table.organization_id, table.host, table.username),
  index("external_registry_organization_id_idx").on(table.organization_id),
  pgPolicy("external_registry_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();

export const registry_access_tokens = pgTable("registry_access_tokens", {
  id: uuid("id").primaryKey(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  token_hash: text("token_hash").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  revoked_at: timestamp("revoked_at", { withTimezone: true, mode: "string" }),
}, (table) => [
  unique("registry_access_tokens_id_organization_id_uidx").on(table.id, table.organization_id),
  uniqueIndex("registry_access_tokens_hash_uidx").on(table.token_hash),
  uniqueIndex("registry_access_tokens_organization_name_uidx")
    .on(table.organization_id, table.name)
    .where(sql`${table.revoked_at} is null`),
  index("registry_access_tokens_organization_id_idx").on(table.organization_id),
  pgPolicy("registry_access_tokens_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();

export const registry_repository_grants = pgTable("registry_repository_grants", {
  id: uuid("id").primaryKey(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  repository_id: uuid("repository_id").notNull(),
  access_token_id: uuid("access_token_id").notNull(),
  can_pull: boolean("can_pull").notNull().default(false),
  can_push: boolean("can_push").notNull().default(false),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("registry_repository_grants_token_repository_uidx").on(table.access_token_id, table.repository_id),
  foreignKey({
    columns: [table.repository_id, table.organization_id],
    foreignColumns: [registry_repositories.id, registry_repositories.organization_id],
    name: "registry_repository_grants_repository_scope_fk",
  }).onDelete("cascade"),
  foreignKey({
    columns: [table.access_token_id, table.organization_id],
    foreignColumns: [registry_access_tokens.id, registry_access_tokens.organization_id],
    name: "registry_repository_grants_token_scope_fk",
  }).onDelete("cascade"),
  index("registry_repository_grants_organization_id_idx").on(table.organization_id),
  index("registry_repository_grants_repository_id_idx").on(table.repository_id),
  index("registry_repository_grants_access_token_id_idx").on(table.access_token_id),
  pgPolicy("registry_repository_grants_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();
