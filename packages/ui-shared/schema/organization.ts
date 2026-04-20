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
import { user } from "./better-auth/studio";
import { relations } from "drizzle-orm";
import { app_tenant, orgAllowed } from "./rls";

export const organization = pgTable(
  "organization",
  {
    id: uuid("id").primaryKey(),
    name: text("name").notNull(),
    email: text("email").notNull().unique(),
    slug: text("slug").notNull().unique(),
    logo: text("logo"),
    created_at: timestamp("created_at").defaultNow().notNull(),
    polar_customer_id: uuid("polar_customer_id").notNull().unique(),
  },
  (table) => [
    uniqueIndex("organization_slug_uidx").on(table.slug),
    index("organization_polar_customer_id_idx").on(table.polar_customer_id),
    index("organization_id_idx").on(table.id),
    pgPolicy("organization_tenant_rls", {
      as: "permissive",
      for: "all",
      to: app_tenant,
      using: orgAllowed(table.id),
      withCheck: orgAllowed(table.id),
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
]).enableRLS();

export const organizationRelations = relations(organization, ({ many }) => ({
  organization_members: many(organization_member),
  organization_invitations: many(organization_invitation),
}));

export const organization_memberRelations = relations(
  organization_member,
  ({ one }) => ({
    organization: one(organization, {
      fields: [organization_member.organization_id],
      references: [organization.id],
    }),
    user: one(user, {
      fields: [organization_member.user_id],
      references: [user.id],
    }),
  }),
);

export const organization_invitationRelations = relations(
  organization_invitation,
  ({ one }) => ({
    organization: one(organization, {
      fields: [organization_invitation.organization_id],
      references: [organization.id],
    }),
    user: one(user, {
      fields: [organization_invitation.inviter_id],
      references: [user.id],
    }),
  }),
);
