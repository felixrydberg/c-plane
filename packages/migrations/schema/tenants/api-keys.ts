import { foreignKey, index, integer, pgEnum, pgPolicy, pgTable, text, timestamp, unique, uuid } from "drizzle-orm/pg-core";
import { organization } from "./organization";
import { app_tenant, orgAllowed } from "../rls";
import { API_KEY_SCOPE_VALUES } from "../../utils/api-key-scopes";

export const api_keys = pgTable("api_keys", {
  id: uuid("id").primaryKey(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
  key_hash: text("key_hash").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
  expires_at: integer("expires_at"),
  allowed_ips: text("allowed_ips"),
}, (table) => [
  unique("api_keys_id_organization_id_uidx").on(table.id, table.organization_id),
  index("api_keys_organization_id_idx").on(table.organization_id),
  index("api_keys_key_hash_idx").on(table.key_hash),
  pgPolicy("api_keys_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();

export const api_key_scopes_type = pgEnum("api_key_scopes_type", API_KEY_SCOPE_VALUES);

export const api_key_scopes = pgTable("api_key_scopes", {
  id: uuid("id").primaryKey(),
  api_key_id: uuid("api_key_id").notNull(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  scope: api_key_scopes_type("scope").notNull(),
  created_at: timestamp("created_at", { withTimezone: true, mode: "string" }).defaultNow().notNull(),
}, (table) => [
  foreignKey({
    columns: [table.api_key_id, table.organization_id],
    foreignColumns: [api_keys.id, api_keys.organization_id],
    name: "api_key_scopes_api_key_scope_fk",
  }).onDelete("cascade"),
  index("api_key_scopes_api_key_id_idx").on(table.api_key_id),
  index("api_key_scopes_scope_idx").on(table.scope),
  index("api_key_scopes_organization_id_idx").on(table.organization_id),
  pgPolicy("api_key_scopes_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();
