import { sql } from "drizzle-orm";
import {
  check,
  foreignKey,
  index,
  pgEnum,
  pgPolicy,
  pgTable,
  text,
  timestamp,
  unique,
  uniqueIndex,
  uuid,
} from "drizzle-orm/pg-core";
import { app_tenant, orgAllowed } from "../rls.ts";
import { organization } from "../tenants/organization.ts";

export const SECRET_SCOPES = ["platform", "tenant"] as const;
export const secret_scope = pgEnum("secret_scope", SECRET_SCOPES);

export const secret = pgTable.withRLS("secret", {
  id: uuid("id").primaryKey(),
  scope: secret_scope("scope").notNull(),
  organization_id: uuid("organization_id")
    .references(() => organization.id, { onDelete: "cascade" }),
  ciphertext: text("ciphertext").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  check("secret_scope_organization_check", sql`(
    (${table.scope} = 'platform' and ${table.organization_id} is null)
    or (${table.scope} = 'tenant' and ${table.organization_id} is not null)
  )`),
  unique("secret_id_scope_organization_uidx").on(
    table.id,
    table.scope,
    table.organization_id,
  ).nullsNotDistinct(),
  unique("secret_id_organization_id_uidx").on(table.id, table.organization_id).nullsNotDistinct(),
  index("secret_organization_id_idx").on(table.organization_id),
  index("secret_scope_idx").on(table.scope),
  pgPolicy("secret_platform_select_rls", {
    as: "permissive",
    for: "select",
    to: app_tenant,
    using: sql`${table.scope} = 'platform'`,
  }),
  pgPolicy("secret_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: sql`${table.scope} = 'tenant' and ${orgAllowed(table.organization_id)}`,
    withCheck: sql`${table.scope} = 'tenant' and ${orgAllowed(table.organization_id)}`,
  }),
]);

export const credential = pgTable.withRLS("credential", {
  id: uuid("id").primaryKey(),
  organization_id: uuid("organization_id")
    .references(() => organization.id, { onDelete: "cascade" }),
  access_key_id: text("access_key_id").notNull(),
  secret_id: uuid("secret_id").notNull(),
  revoked_at: timestamp("revoked_at", { withTimezone: true, mode: "string" }),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  updated_at: timestamp("updated_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  uniqueIndex("credential_access_key_id_uidx").on(table.access_key_id),
  uniqueIndex("credential_secret_id_uidx").on(table.secret_id),
  unique("credential_id_organization_id_uidx").on(table.id, table.organization_id).nullsNotDistinct(),
  foreignKey({
    columns: [table.secret_id],
    foreignColumns: [secret.id],
    name: "credential_secret_id_secret_id_fkey",
  }).onDelete("restrict"),
  foreignKey({
    columns: [table.secret_id, table.organization_id],
    foreignColumns: [secret.id, secret.organization_id],
    name: "credential_secret_id_fk",
  }).onDelete("restrict"),
  index("credential_organization_id_idx").on(table.organization_id),
  index("credential_revoked_at_idx").on(table.revoked_at),
  pgPolicy("credential_platform_select_rls", {
    as: "permissive",
    for: "select",
    to: app_tenant,
    using: sql`${table.organization_id} is null`,
  }),
  pgPolicy("credential_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);
