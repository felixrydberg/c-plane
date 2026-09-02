import { index, pgPolicy, pgTable, text, timestamp, unique, uniqueIndex, uuid, foreignKey } from "drizzle-orm/pg-core";
import { project } from "./index.ts";
import { organization } from "../tenants/organization.ts";
import { app_tenant, orgAllowed } from "../rls.ts";
import { bucket } from "../infrastructure/buckets.ts";
import { credential } from "../infrastructure/secrets.ts";
import { region } from "../infrastructure/regions.ts";

export const storage_bucket = pgTable.withRLS('storage_bucket', {
  id: uuid("id").primaryKey(),
  project_id: uuid("project_id")
    .notNull()
    .references(() => project.id, { onDelete: "cascade" }),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  region_id: uuid("region_id")
    .notNull()
    .references(() => region.id, { onDelete: "restrict" }),
  bucket_id: uuid("bucket_id").notNull().references(() => bucket.id, { onDelete: "restrict" }),
  name: text("name").notNull(),
}, (table) => [
  uniqueIndex("storage_bucket_project_name_uidx").on(table.project_id, table.name),
  unique("storage_bucket_id_organization_id_uidx").on(table.id, table.organization_id),
  uniqueIndex("storage_bucket_foundation_bucket_uidx").on(table.bucket_id),
  index("storage_bucket_project_id_idx").on(table.project_id),
  index("storage_bucket_organization_id_idx").on(table.organization_id),
  index("storage_bucket_region_id_idx").on(table.region_id),
  pgPolicy("storage_bucket_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);

export const storage_access_token = pgTable.withRLS("storage_access_token", {
  credential_id: uuid("credential_id").primaryKey(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
  project_id: uuid("project_id")
    .notNull()
    .references(() => project.id, { onDelete: "cascade" }),
  name: text("name").notNull(),
}, (table) => [
  unique("storage_access_token_credential_organization_uidx").on(table.credential_id, table.organization_id),
  // unique name per project enforced in-app (active_token_named): revoked_at lives on credential, partial index can't join
  index("storage_access_token_organization_id_idx").on(table.organization_id),
  index("storage_access_token_project_id_idx").on(table.project_id),
  foreignKey({
    columns: [table.credential_id, table.organization_id],
    foreignColumns: [credential.id, credential.organization_id],
    name: "storage_access_token_credential_scope_fk",
  }).onDelete("cascade"),
  pgPolicy("storage_access_token_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]);
