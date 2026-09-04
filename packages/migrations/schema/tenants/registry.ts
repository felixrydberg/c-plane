import { bigint, boolean, foreignKey, index, pgEnum, pgPolicy, pgTable, text, timestamp, unique, uniqueIndex, uuid } from "drizzle-orm/pg-core";
import { sql } from "drizzle-orm";
import { app_tenant, orgAllowed } from "../rls.ts";
import { organization } from "./organization.ts";
import { bucket } from "../infrastructure/buckets.ts";
import { credential } from "../infrastructure/secrets.ts";
import { worker_queue } from "../infrastructure/worker-queue.ts";
import { project } from "../projects/index.ts";

export const managed_registry_status = pgEnum("managed_registry_status", ["active", "maintenance"]);

export const managed_registry = pgTable.withRLS("managed_registry", {
  organization_id: uuid("organization_id")
    .primaryKey()
    .references(() => organization.id, { onDelete: "cascade" }),
  bucket_id: uuid("bucket_id")
    .notNull()
    .references(() => bucket.id, { onDelete: "restrict" }),
  credential_id: uuid("credential_id").notNull(),
  status: managed_registry_status("status").notNull().default("active"),
  gc_active_job_id: uuid("gc_active_job_id").references(() => worker_queue.id, { onDelete: "set null" }),
  storage_revision: uuid("storage_revision").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("managed_registry_bucket_id_uidx").on(table.bucket_id),
  uniqueIndex("managed_registry_credential_id_uidx").on(table.credential_id),
  index("managed_registry_status_idx").on(table.status),
  index("managed_registry_gc_active_job_idx").on(table.gc_active_job_id),
  foreignKey({
    columns: [table.credential_id, table.organization_id],
    foreignColumns: [credential.id, credential.organization_id],
    name: "managed_registry_credential_scope_fk",
  }).onDelete("cascade"),
  pgPolicy("managed_registry_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);

export const managed_registry_gc_runs = pgTable.withRLS("managed_registry_gc_runs", {
  id: uuid("id").primaryKey(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => managed_registry.organization_id, { onDelete: "cascade" }),
  started_at: timestamp("started_at", { withTimezone: true, mode: "string" }).notNull(),
  finished_at: timestamp("finished_at", { withTimezone: true, mode: "string" }).notNull(),
  bytes_before: bigint("bytes_before", { mode: "number" }),
  bytes_after: bigint("bytes_after", { mode: "number" }),
  result: text("result").notNull(),
  error: text("error"),
}, (table) => [
  index("managed_registry_gc_runs_organization_id_idx").on(table.organization_id),
  pgPolicy("managed_registry_gc_runs_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);

export const registry_repositories = pgTable.withRLS("registry_repositories", {
  id: uuid("id").primaryKey(),
  project_id: uuid("project_id").notNull(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  unique("registry_repositories_id_project_organization_uidx").on(table.id, table.project_id, table.organization_id),
  uniqueIndex("registry_repositories_project_name_uidx").on(table.project_id, table.name),
  index("registry_repositories_project_id_idx").on(table.project_id),
  index("registry_repositories_organization_id_idx").on(table.organization_id),
  foreignKey({
    columns: [table.project_id, table.organization_id],
    foreignColumns: [project.id, project.organization_id],
    name: "registry_repositories_project_scope_fk",
  }).onDelete("cascade"),
  pgPolicy("registry_repositories_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);

export const external_registry = pgTable.withRLS("external_registry", {
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
]);

export const registry_access_tokens = pgTable.withRLS("registry_access_tokens", {
  id: uuid("id").primaryKey(),
  project_id: uuid("project_id").notNull(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  token_hash: text("token_hash").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  revoked_at: timestamp("revoked_at", { withTimezone: true, mode: "string" }),
}, (table) => [
  unique("registry_access_tokens_id_project_organization_uidx").on(table.id, table.project_id, table.organization_id),
  uniqueIndex("registry_access_tokens_hash_uidx").on(table.token_hash),
  uniqueIndex("registry_access_tokens_project_name_uidx")
    .on(table.project_id, table.name)
    .where(sql`${table.revoked_at} is null`),
  index("registry_access_tokens_project_id_idx").on(table.project_id),
  index("registry_access_tokens_organization_id_idx").on(table.organization_id),
  foreignKey({
    columns: [table.project_id, table.organization_id],
    foreignColumns: [project.id, project.organization_id],
    name: "registry_access_tokens_project_scope_fk",
  }).onDelete("cascade"),
  pgPolicy("registry_access_tokens_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);

export const registry_repository_grants = pgTable.withRLS("registry_repository_grants", {
  id: uuid("id").primaryKey(),
  project_id: uuid("project_id").notNull(),
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
    columns: [table.repository_id, table.project_id, table.organization_id],
    foreignColumns: [registry_repositories.id, registry_repositories.project_id, registry_repositories.organization_id],
    name: "registry_repository_grants_repository_scope_fk",
  }).onDelete("cascade"),
  foreignKey({
    columns: [table.access_token_id, table.project_id, table.organization_id],
    foreignColumns: [registry_access_tokens.id, registry_access_tokens.project_id, registry_access_tokens.organization_id],
    name: "registry_repository_grants_token_scope_fk",
  }).onDelete("cascade"),
  index("registry_repository_grants_project_id_idx").on(table.project_id),
  index("registry_repository_grants_organization_id_idx").on(table.organization_id),
  index("registry_repository_grants_repository_id_idx").on(table.repository_id),
  index("registry_repository_grants_access_token_id_idx").on(table.access_token_id),
  foreignKey({
    columns: [table.project_id, table.organization_id],
    foreignColumns: [project.id, project.organization_id],
    name: "registry_repository_grants_project_scope_fk",
  }).onDelete("cascade"),
  pgPolicy("registry_repository_grants_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);
