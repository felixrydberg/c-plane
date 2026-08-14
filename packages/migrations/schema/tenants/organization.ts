import {
  pgTable,
  pgPolicy,
  text,
  timestamp,
  index,
  uniqueIndex,
  pgEnum,
  uuid,
} from "drizzle-orm/pg-core";
import { sql } from "drizzle-orm";
import { user } from "./studio";
import { app_tenant, orgAllowed } from "../rls";

export const organization = pgTable(
  "organization",
  {
    id: uuid("id").primaryKey(),
    name: text("name").notNull(),
    email: text("email").notNull().unique(),
    slug: text("slug").notNull().unique(),
    logo: text("logo"),
    created_at: timestamp("created_at").defaultNow().notNull(),
  },
  (table) => [
    uniqueIndex("organization_slug_uidx").on(table.slug),
    index("organization_id_idx").on(table.id),
    pgPolicy("organization_tenant_rls_select", {
      as: "permissive",
      for: "select",
      to: app_tenant,
      using: orgAllowed(table.id),
    }),
    pgPolicy("organization_tenant_rls_update", {
      as: "permissive",
      for: "update",
      to: app_tenant,
      using: orgAllowed(table.id),
      withCheck: orgAllowed(table.id),
    }),
    pgPolicy("organization_tenant_rls_delete", {
      as: "permissive",
      for: "delete",
      to: app_tenant,
      using: orgAllowed(table.id),
    }),
    pgPolicy("organization_tenant_rls_insert", {
      as: "permissive",
      for: "insert",
      to: app_tenant,
      withCheck: sql`true`,
    }),
  ],
).enableRLS();

export const organization_member = pgTable(
  "organization_member",
  {
    id: uuid("id").primaryKey(),
    organization_id: uuid("organization_id")
      .notNull()
      .references(() => organization.id, { onDelete: "cascade" }),
    user_id: uuid("user_id")
      .notNull()
      .references(() => user.id, { onDelete: "cascade" }),
    role: text("role").default("member").notNull(),
    created_at: timestamp("created_at").defaultNow().notNull(),
  },
  (table) => [
    uniqueIndex("organization_member_user_id_organization_id_uidx").on(
      table.user_id,
      table.organization_id,
    ),
    index("organization_member_organization_id_idx").on(table.organization_id),
    index("organization_member_user_id_idx").on(table.user_id),
    pgPolicy("organization_member_tenant_rls", {
      as: "permissive",
      for: "all",
      to: app_tenant,
      using: orgAllowed(table.organization_id),
      withCheck: orgAllowed(table.organization_id),
    }),
  ],
).enableRLS();

export const organization_invitation_status = pgEnum(
  "organization_invitation_status",
  ["pending", "accepted", "declined", "revoked"],
);

export const organization_invitation = pgTable(
  "organization_invitation",
  {
    id: uuid("id").primaryKey(),
    organization_id: uuid("organization_id")
      .notNull()
      .references(() => organization.id, { onDelete: "cascade" }),
    email: text("email").notNull(),
    role: text("role").notNull().default("member"),
    status: organization_invitation_status("status").default("pending").notNull(),
    expires_at: timestamp("expires_at").notNull(),
    created_at: timestamp("created_at").defaultNow().notNull(),
    inviter_id: uuid("inviter_id")
      .notNull()
      .references(() => user.id, { onDelete: "cascade" }),
  },
  (table) => [
    index("organization_invitation_organization_id_idx").on(
      table.organization_id,
    ),
    index("organization_invitation_email_idx").on(table.email),
    pgPolicy("organization_invitation_tenant_rls", {
      as: "permissive",
      for: "all",
      to: app_tenant,
      using: orgAllowed(table.organization_id),
      withCheck: orgAllowed(table.organization_id),
    }),
  ],
).enableRLS();

export const active_organization = pgTable("active_organization", {
  user_id: uuid("user_id")
    .primaryKey()
    .references(() => user.id, { onDelete: "cascade" }).unique(),
  organization_id: uuid("organization_id")
    .notNull()
    .references(() => organization.id, { onDelete: "cascade" }),
}, (table) => [
  index("active_organization_user_id_idx").on(table.user_id),
  pgPolicy("active_organization_tenant_rls", {
    as: "permissive",
    for: "all",
    to: app_tenant,
    using: orgAllowed(table.organization_id),
    withCheck: orgAllowed(table.organization_id),
  }),
]).enableRLS();
