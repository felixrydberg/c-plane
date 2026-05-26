import { relations, sql } from "drizzle-orm";
import {
  pgTable,
  text,
  timestamp,
  boolean,
  uuid,
  index,
} from "drizzle-orm/pg-core";

export const cplane_user = pgTable("cplane_user", {
  id: uuid("id")
    .default(sql`pg_catalog.gen_random_uuid()`)
    .primaryKey(),
  name: text("name").notNull(),
  email: text("email").notNull().unique(),
  emailVerified: boolean("email_verified").default(false).notNull(),
  image: text("image"),
  createdAt: timestamp("created_at").defaultNow().notNull(),
  updatedAt: timestamp("updated_at")
    .defaultNow()
    .$onUpdate(() => /* @__PURE__ */ new Date())
    .notNull(),
  role: text("role"),
  banned: boolean("banned").default(false),
  banReason: text("ban_reason"),
  banExpires: timestamp("ban_expires"),
  twoFactorEnabled: boolean("two_factor_enabled").default(false),
});

export const cplane_account = pgTable(
  "cplane_account",
  {
    id: uuid("id")
      .default(sql`pg_catalog.gen_random_uuid()`)
      .primaryKey(),
    accountId: text("account_id").notNull(),
    providerId: text("provider_id").notNull(),
    userId: uuid("user_id")
      .notNull()
      .references(() => cplane_user.id, { onDelete: "cascade" }),
    accessToken: text("access_token"),
    refreshToken: text("refresh_token"),
    idToken: text("id_token"),
    accessTokenExpiresAt: timestamp("access_token_expires_at"),
    refreshTokenExpiresAt: timestamp("refresh_token_expires_at"),
    scope: text("scope"),
    password: text("password"),
    createdAt: timestamp("created_at").defaultNow().notNull(),
    updatedAt: timestamp("updated_at")
      .$onUpdate(() => /* @__PURE__ */ new Date())
      .notNull(),
  },
  (table) => [index("cplane_account_userId_idx").on(table.userId)],
);

export const cplane_auth_verification = pgTable(
  "cplane_auth_verification",
  {
    id: uuid("id")
      .default(sql`pg_catalog.gen_random_uuid()`)
      .primaryKey(),
    identifier: text("identifier").notNull(),
    value: text("value").notNull(),
    expiresAt: timestamp("expires_at").notNull(),
    createdAt: timestamp("created_at").defaultNow().notNull(),
    updatedAt: timestamp("updated_at")
      .defaultNow()
      .$onUpdate(() => /* @__PURE__ */ new Date())
      .notNull(),
  },
  (table) => [
    index("cplane_auth_verification_identifier_idx").on(table.identifier),
  ],
);

export const cplane_two_factor = pgTable(
  "cplane_two_factor",
  {
    id: uuid("id")
      .default(sql`pg_catalog.gen_random_uuid()`)
      .primaryKey(),
    secret: text("secret").notNull(),
    backupCodes: text("backup_codes").notNull(),
    userId: uuid("user_id")
      .notNull()
      .references(() => cplane_user.id, { onDelete: "cascade" }),
  },
  (table) => [
    index("cplane_two_factor_secret_idx").on(table.secret),
    index("cplane_two_factor_userId_idx").on(table.userId),
  ],
);

export const cplane_userRelations = relations(cplane_user, ({ many }) => ({
  cplane_accounts: many(cplane_account),
  cplane_two_factors: many(cplane_two_factor),
}));

export const cplane_accountRelations = relations(cplane_account, ({ one }) => ({
  cplane_user: one(cplane_user, {
    fields: [cplane_account.userId],
    references: [cplane_user.id],
  }),
}));

export const cplane_two_factorRelations = relations(
  cplane_two_factor,
  ({ one }) => ({
    cplane_user: one(cplane_user, {
      fields: [cplane_two_factor.userId],
      references: [cplane_user.id],
    }),
  }),
);
